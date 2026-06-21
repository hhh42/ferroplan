//! A native, Blockly-style problem editor: build a PDDL problem by assembling
//! object/fact blocks with click fields (no syntax, no text typing). Objects are
//! auto-named; clicking a block field cycles it through the valid choices
//! (type / predicate / type-compatible object). "Apply" regenerates the problem
//! (`viz::to_pddl`) and revisualizes it via `Scene::load_src`; "Export" writes it.
//!
//! `bevy_ui` has no dropdown widget, so fields are buttons that cycle on click;
//! the panel is rebuilt from `Editor` whenever it changes.

use std::collections::HashMap;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use ferroplan::types::{Action, Domain, Effect, Formula, Term};
use ferroplan::viz;

use crate::scene::Scene;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Problem,
    Domain,
}

/// Which text field currently captures keyboard input.
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // *Name fields are clearer than bare names here
pub enum Focus {
    DomainName,
    TypeName(usize),
    PredName(usize),
    ActionName(usize),
}

/// A literal in a precondition/effect: `(neg, predicate, [arg vars])`.
type Lit = (bool, String, Vec<String>);

/// A conditional effect: `(when-condition literals, when-effect literals)`.
type When = (Vec<Lit>, Vec<Lit>);

/// Where a literal lives inside an action (used by the edit actions).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LitLoc {
    Pre,
    Eff,
    WhenCond(usize),
    WhenEff(usize),
}

/// An editor action. Actions built from literals + `or`-preconditions + flat
/// `when` conditional effects are modeled and editable; anything richer
/// (forall/exists/numeric/nested) is preserved verbatim.
enum EdAction {
    Modeled {
        name: String,
        params: Vec<(String, String)>, // (?var, type)
        pre: Vec<Lit>,
        pre_or: bool, // precondition is (or …) instead of (and …)
        eff: Vec<Lit>,
        whens: Vec<When>, // (when (and cond) (and eff))
    },
    Raw(String),
}

#[derive(Resource, Default)]
pub struct Editor {
    pub open: bool,
    pub focus: Option<Focus>,
    mode: Mode,
    dirty: bool,
    status: String,

    // problem side
    problem_name: String,
    objects: Vec<(String, String)>,   // (name, type)
    init: Vec<(String, Vec<String>)>, // (pred, args)
    goal: Vec<(String, Vec<String>)>, // (pred, args)
    counters: HashMap<String, u32>,
    seeded: bool,

    // domain side
    dname: String,
    requirements: String,               // raw s-expr, preserved
    types: Vec<(String, String)>,       // (type, parent)
    dpreds: Vec<(String, Vec<String>)>, // (name, arg types)
    actions: Vec<EdAction>,
    dseeded: bool,
}

#[derive(Component)]
pub struct EditorRoot;

/// A draggable fact block (its list + index at build time).
#[derive(Component, Clone, Copy)]
pub enum DragKind {
    Init(usize),
    Goal(usize),
}

/// A drop target column.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Init,
    Goal,
}

/// The label that follows the cursor while dragging.
#[derive(Component)]
pub struct Ghost;

#[derive(Resource, Default)]
pub struct Drag {
    held: Option<DragKind>,
    ghost: Option<Entity>,
}

/// Move a dragged fact into the zone it was dropped on (cross-zone only; a drop
/// back on the same zone is a no-op). Pure — unit-tested.
fn resolve_drop(held: DragKind, zone: Zone, editor: &mut Editor) -> bool {
    match (held, zone) {
        (DragKind::Init(i), Zone::Goal) if i < editor.init.len() => {
            let f = editor.init.remove(i);
            editor.goal.push(f);
            true
        }
        (DragKind::Goal(i), Zone::Init) if i < editor.goal.len() => {
            let f = editor.goal.remove(i);
            editor.init.push(f);
            true
        }
        _ => false,
    }
}

#[derive(Component, Clone)]
pub enum Act {
    // problem
    AddObject,
    RemoveObject(usize),
    CycleType(usize),
    AddFact(bool), // goal?
    RemoveFact(bool, usize),
    CyclePred(bool, usize),
    CycleArg(bool, usize, usize), // goal?, fact, slot
    // domain
    AddType,
    RemoveType(usize),
    CycleSuper(usize),
    AddPred,
    RemovePred(usize),
    AddArg(usize),
    RemoveArg(usize),
    CycleArgType(usize, usize), // pred, slot
    // actions  (the bool is `eff`: false = precondition, true = effect)
    AddAction,
    RemoveAction(usize),
    AddParam(usize),
    RemoveParam(usize),
    CycleParamType(usize, usize), // action, param
    TogglePreKind(usize),         // and <-> or
    AddWhen(usize),
    RemoveWhen(usize, usize),
    AddLit(usize, LitLoc),
    RemoveLit(usize, LitLoc, usize),
    CycleLitPred(usize, LitLoc, usize),
    CycleLitArg(usize, LitLoc, usize, usize), // action, loc, lit, slot
    ToggleNeg(usize, LitLoc, usize),
    // shared
    SetFocus(Focus),
    ToggleMode,
    Apply,
    Export,
    Close,
}

pub fn toggle_editor(
    keys: Res<ButtonInput<KeyCode>>,
    scene: Res<Scene>,
    mut editor: ResMut<Editor>,
) {
    if editor.focus.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        editor.open = !editor.open;
        if editor.open && !editor.seeded {
            seed(&mut editor, &scene);
        }
        editor.dirty = true;
    }
    // Tab toggles Problem <-> Domain while the editor is open.
    if editor.open && keys.just_pressed(KeyCode::Tab) {
        editor.mode = if editor.mode == Mode::Problem {
            Mode::Domain
        } else {
            Mode::Problem
        };
        if editor.mode == Mode::Domain && !editor.dseeded {
            seed_domain(&mut editor, &scene);
        }
        editor.dirty = true;
    }
}

/// Type into the focused text field (domain/type/predicate names). Captures all
/// keys while a field is focused, so global shortcuts are suppressed meanwhile.
pub fn text_input(mut evr: EventReader<KeyboardInput>, mut editor: ResMut<Editor>) {
    if editor.focus.is_none() {
        evr.clear();
        return;
    }
    let mut changed = false;
    let mut defocus = false;
    let mut edits: Vec<TextEdit> = Vec::new();
    for ev in evr.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        match &ev.logical_key {
            Key::Character(s) => {
                for c in s.chars() {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                        edits.push(TextEdit::Push(c.to_ascii_lowercase()));
                    }
                }
            }
            Key::Backspace => edits.push(TextEdit::Pop),
            Key::Enter | Key::Escape => defocus = true,
            _ => {}
        }
    }
    if let Some(focus) = editor.focus {
        if let Some(field) = focused_field_mut(&mut editor, focus) {
            for e in edits {
                match e {
                    TextEdit::Push(c) => field.push(c),
                    TextEdit::Pop => {
                        field.pop();
                    }
                }
                changed = true;
            }
        }
    }
    if defocus {
        editor.focus = None;
        changed = true;
    }
    if changed {
        editor.dirty = true;
    }
}

enum TextEdit {
    Push(char),
    Pop,
}

/// Drag a fact block (by its grip) and drop it on the other zone to move it
/// between Init and Goal. A ghost label follows the cursor while dragging.
#[allow(clippy::too_many_arguments)]
pub fn editor_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut drag: ResMut<Drag>,
    mut editor: ResMut<Editor>,
    grips: Query<(&DragKind, &RelativeCursorPosition)>,
    zones: Query<(&Zone, &RelativeCursorPosition)>,
    mut ghosts: Query<&mut Node, With<Ghost>>,
    mut commands: Commands,
) {
    if !editor.open || editor.focus.is_some() {
        return;
    }
    let cursor = windows.get_single().ok().and_then(|w| w.cursor_position());

    if mouse.just_pressed(MouseButton::Left) && drag.held.is_none() {
        if let Some(pos) = cursor {
            if let Some((kind, _)) = grips.iter().find(|(_, r)| r.mouse_over()) {
                drag.held = Some(*kind);
                let id = commands
                    .spawn((
                        Ghost,
                        Text::new(ghost_text(&editor, *kind)),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.9, 0.5)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(pos.x + 8.0),
                            top: Val::Px(pos.y + 4.0),
                            ..default()
                        },
                        GlobalZIndex(1000),
                    ))
                    .id();
                drag.ghost = Some(id);
            }
        }
    }

    if drag.held.is_some() {
        if let (Some(pos), Some(g)) = (cursor, drag.ghost) {
            if let Ok(mut node) = ghosts.get_mut(g) {
                node.left = Val::Px(pos.x + 8.0);
                node.top = Val::Px(pos.y + 4.0);
            }
        }
    }

    if mouse.just_released(MouseButton::Left) {
        if let Some(held) = drag.held.take() {
            if let Some((zone, _)) = zones.iter().find(|(_, r)| r.mouse_over()) {
                if resolve_drop(held, *zone, &mut editor) {
                    editor.dirty = true;
                }
            }
            if let Some(g) = drag.ghost.take() {
                commands.entity(g).despawn_recursive();
            }
        }
    }
}

fn ghost_text(editor: &Editor, kind: DragKind) -> String {
    let lit = match kind {
        DragKind::Init(i) => editor.init.get(i),
        DragKind::Goal(i) => editor.goal.get(i),
    };
    lit.map(|(p, a)| format!("({} {})", p, a.join(" ")))
        .unwrap_or_default()
}

/// Scroll the editor panel with the mouse wheel (the panel can be taller than the
/// window). Only active while the editor is open.
pub fn scroll_editor(
    mut wheel: EventReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    editor: Res<Editor>,
    mut q: Query<&mut ScrollPosition, With<EditorRoot>>,
) {
    if !editor.open {
        wheel.clear();
        return;
    }
    let mut dy = 0.0;
    for ev in wheel.read() {
        dy += match ev.unit {
            MouseScrollUnit::Line => ev.y * 24.0,
            MouseScrollUnit::Pixel => ev.y,
        };
    }
    if editor.focus.is_none() {
        if keys.just_pressed(KeyCode::PageDown) {
            dy -= 240.0;
        }
        if keys.just_pressed(KeyCode::PageUp) {
            dy += 240.0;
        }
    }
    if dy != 0.0 {
        if let Ok(mut sp) = q.get_single_mut() {
            sp.offset_y = (sp.offset_y - dy).max(0.0);
        }
    }
}

fn focused_field_mut(editor: &mut Editor, focus: Focus) -> Option<&mut String> {
    match focus {
        Focus::DomainName => Some(&mut editor.dname),
        Focus::TypeName(i) => editor.types.get_mut(i).map(|t| &mut t.0),
        Focus::PredName(i) => editor.dpreds.get_mut(i).map(|p| &mut p.0),
        Focus::ActionName(i) => match editor.actions.get_mut(i) {
            Some(EdAction::Modeled { name, .. }) => Some(name),
            _ => None,
        },
    }
}

fn seed(editor: &mut Editor, scene: &Scene) {
    if let Some(d) = &scene.domain {
        editor.dname = d.name.to_lowercase();
    }
    if let Some(p) = &scene.problem {
        editor.problem_name = p.name.to_lowercase();
        editor.objects = p
            .objects
            .iter()
            .map(|(o, t)| (o.to_lowercase(), t.to_lowercase()))
            .collect();
        editor.init = p
            .init_atoms
            .iter()
            .map(|(pr, a)| {
                (
                    pr.to_lowercase(),
                    a.iter().map(|x| x.to_lowercase()).collect(),
                )
            })
            .collect();
        editor.goal = viz::goal_facts(p);
    }
    editor.seeded = true;
}

fn seed_domain(editor: &mut Editor, scene: &Scene) {
    if let Some(d) = &scene.domain {
        editor.dname = d.name.to_lowercase();
        editor.types = d
            .types
            .iter()
            .map(|t| {
                let parent = d
                    .type_parent
                    .iter()
                    .find(|(c, _)| c.eq_ignore_ascii_case(t))
                    .map(|(_, p)| p.to_lowercase())
                    .unwrap_or_default();
                (t.to_lowercase(), parent)
            })
            .collect();
        editor.dpreds = d
            .predicates
            .iter()
            .map(|(n, a)| {
                (
                    n.to_lowercase(),
                    a.iter().map(|x| x.to_lowercase()).collect(),
                )
            })
            .collect();
    }
    editor.requirements = extract_block(&scene.domain_src, "(:requirements").unwrap_or_default();
    let raws = extract_all_blocks(&scene.domain_src, "(:action");
    editor.actions = scene
        .domain
        .as_ref()
        .map(|d| {
            d.actions
                .iter()
                .enumerate()
                .map(|(i, a)| seed_action(a, raws.get(i)))
                .collect()
        })
        .unwrap_or_default();
    editor.dseeded = true;
}

/// Model an action if its precond/effect are flat (conjunction of literals);
/// otherwise keep the raw text.
fn seed_action(a: &Action, raw: Option<&String>) -> EdAction {
    match (flatten_pre(&a.precond), flatten_eff(&a.effect)) {
        (Some((pre_or, pre)), Some((eff, whens))) => EdAction::Modeled {
            name: a.name.to_lowercase(),
            params: a
                .params
                .iter()
                .map(|(n, t)| (param_norm(n), t.to_lowercase()))
                .collect(),
            pre,
            pre_or,
            eff,
            whens,
        },
        _ => EdAction::Raw(raw.cloned().unwrap_or_default()),
    }
}

fn param_norm(s: &str) -> String {
    let s = s.to_lowercase();
    if s.starts_with('?') {
        s
    } else {
        format!("?{s}")
    }
}

fn term_str(t: &Term) -> String {
    match t {
        Term::Var(s) => param_norm(s),
        Term::Const(s) => s.to_lowercase(),
    }
}

/// A single literal (atom or negated atom), else None.
fn flat_lit(f: &Formula) -> Option<Lit> {
    match f {
        Formula::Atom(p, ts) => Some((false, p.to_lowercase(), ts.iter().map(term_str).collect())),
        Formula::Not(b) => match &**b {
            Formula::Atom(p, ts) => {
                Some((true, p.to_lowercase(), ts.iter().map(term_str).collect()))
            }
            _ => None,
        },
        _ => None,
    }
}

/// A flat precondition: `(is_or, literals)`. None if it nests beyond and/or of literals.
fn flatten_pre(f: &Formula) -> Option<(bool, Vec<Lit>)> {
    match f {
        Formula::True => Some((false, vec![])),
        Formula::And(fs) => fs
            .iter()
            .map(flat_lit)
            .collect::<Option<_>>()
            .map(|l| (false, l)),
        Formula::Or(fs) => fs
            .iter()
            .map(flat_lit)
            .collect::<Option<_>>()
            .map(|l| (true, l)),
        _ => flat_lit(f).map(|l| (false, vec![l])),
    }
}

/// A flat effect: `(unconditional literals, [(when-cond, when-eff)])`. None if it
/// uses forall / numeric / nested whens.
fn flatten_eff(e: &Effect) -> Option<(Vec<Lit>, Vec<When>)> {
    let mut lits = Vec::new();
    let mut whens = Vec::new();
    collect_eff(e, &mut lits, &mut whens)?;
    Some((lits, whens))
}

fn collect_eff(e: &Effect, lits: &mut Vec<Lit>, whens: &mut Vec<When>) -> Option<()> {
    match e {
        Effect::And(es) => {
            for x in es {
                collect_eff(x, lits, whens)?;
            }
            Some(())
        }
        Effect::Add(p, ts) => {
            lits.push((false, p.to_lowercase(), ts.iter().map(term_str).collect()));
            Some(())
        }
        Effect::Del(p, ts) => {
            lits.push((true, p.to_lowercase(), ts.iter().map(term_str).collect()));
            Some(())
        }
        Effect::When(cond, eff) => {
            let (or, clits) = flatten_pre(cond)?;
            if or {
                return None; // a when-condition must be a conjunction
            }
            let (elits, ewhens) = flatten_eff(eff)?;
            if !ewhens.is_empty() {
                return None; // no nested whens
            }
            whens.push((clits, elits));
            Some(())
        }
        _ => None,
    }
}

fn lit_str(l: &Lit) -> String {
    let (neg, pred, args) = l;
    let inner = if args.is_empty() {
        format!("({pred})")
    } else {
        format!("({} {})", pred, args.join(" "))
    };
    if *neg {
        format!("(not {inner})")
    } else {
        inner
    }
}

/// Render literals grouped under `and`/`or`. A single `and` literal is bare.
fn lits_grouped(lits: &[Lit], or: bool) -> String {
    let kw = if or { "or" } else { "and" };
    match lits.len() {
        0 => format!("({kw})"),
        1 if !or => lit_str(&lits[0]),
        _ => format!(
            "({} {})",
            kw,
            lits.iter().map(lit_str).collect::<Vec<_>>().join(" ")
        ),
    }
}

fn action_to_pddl(a: &EdAction) -> String {
    match a {
        EdAction::Raw(s) => s.clone(),
        EdAction::Modeled {
            name,
            params,
            pre,
            pre_or,
            eff,
            whens,
        } => {
            let ps = params
                .iter()
                .map(|(n, t)| format!("{n} - {t}"))
                .collect::<Vec<_>>()
                .join(" ");
            let mut eff_parts: Vec<String> = eff.iter().map(lit_str).collect();
            for (cond, weff) in whens {
                eff_parts.push(format!(
                    "(when {} {})",
                    lits_grouped(cond, false),
                    lits_grouped(weff, false)
                ));
            }
            let eff_s = match eff_parts.len() {
                0 => "(and)".to_string(),
                1 => eff_parts.remove(0),
                _ => format!("(and {})", eff_parts.join(" ")),
            };
            format!(
                "(:action {}\n   :parameters ({})\n   :precondition {}\n   :effect {})",
                name,
                ps,
                lits_grouped(pre, *pre_or),
                eff_s
            )
        }
    }
}

/// The literal list addressed by `loc` within a modeled action.
fn lits_at_mut(act: &mut EdAction, loc: LitLoc) -> Option<&mut Vec<Lit>> {
    if let EdAction::Modeled {
        pre, eff, whens, ..
    } = act
    {
        match loc {
            LitLoc::Pre => Some(pre),
            LitLoc::Eff => Some(eff),
            LitLoc::WhenCond(i) => whens.get_mut(i).map(|w| &mut w.0),
            LitLoc::WhenEff(i) => whens.get_mut(i).map(|w| &mut w.1),
        }
    } else {
        None
    }
}

fn action_params(editor: &Editor, i: usize) -> Vec<(String, String)> {
    match editor.actions.get(i) {
        Some(EdAction::Modeled { params, .. }) => params.clone(),
        _ => vec![],
    }
}

fn ed_is_subtype(types: &[(String, String)], ty: &str, of: &str) -> bool {
    if of.eq_ignore_ascii_case("object") || ty.eq_ignore_ascii_case(of) {
        return true;
    }
    let mut cur = ty.to_string();
    for _ in 0..64 {
        if cur.eq_ignore_ascii_case(of) {
            return true;
        }
        match types.iter().find(|(c, _)| c.eq_ignore_ascii_case(&cur)) {
            Some((_, p)) if !p.is_empty() => cur = p.clone(),
            _ => return false,
        }
    }
    false
}

/// Action params whose type fits an argument slot of type `arg_ty`.
fn compatible_params(
    params: &[(String, String)],
    types: &[(String, String)],
    arg_ty: &str,
) -> Vec<String> {
    params
        .iter()
        .filter(|(_, t)| ed_is_subtype(types, t, arg_ty))
        .map(|(n, _)| n.clone())
        .collect()
}

fn first_compatible_param(
    params: &[(String, String)],
    types: &[(String, String)],
    arg_ty: &str,
) -> String {
    compatible_params(params, types, arg_ty)
        .into_iter()
        .next()
        .unwrap_or_else(|| "?".into())
}

/// First balanced `(...)` block beginning with `prefix` (case-insensitive).
fn extract_block(src: &str, prefix: &str) -> Option<String> {
    let low = src.to_lowercase();
    let start = low.find(&prefix.to_lowercase())?;
    balanced_from(src, start)
}

fn extract_all_blocks(src: &str, prefix: &str) -> Vec<String> {
    let low = src.to_lowercase();
    let p = prefix.to_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = low[from..].find(&p) {
        let start = from + rel;
        match balanced_from(src, start) {
            Some(block) => {
                from = start + block.len();
                out.push(block);
            }
            None => break,
        }
    }
    out
}

/// The balanced parenthesized block starting at/after `start` (skips `;` comments).
fn balanced_from(src: &str, start: usize) -> Option<String> {
    let b = src.as_bytes();
    let mut i = start;
    while i < b.len() && b[i] != b'(' {
        i += 1;
    }
    let open = i;
    let mut depth = 0i32;
    while i < b.len() {
        match b[i] {
            b';' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(src[open..=i].to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

// ---- click handling ----

#[allow(clippy::type_complexity)] // Bevy query filters are inherently verbose
pub fn handle_clicks(
    interactions: Query<(&Interaction, &Act), (Changed<Interaction>, With<Button>)>,
    mut editor: ResMut<Editor>,
    mut scene: ResMut<Scene>,
) {
    for (interaction, act) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_act(act, &mut editor, &mut scene);
        editor.dirty = true;
    }
}

fn apply_act(act: &Act, editor: &mut Editor, scene: &mut Scene) {
    let domain = scene.domain.clone(); // small; avoids borrow tangle with load_src
    let types: Vec<String> = domain
        .as_ref()
        .map(|d| d.types.iter().map(|t| t.to_lowercase()).collect())
        .unwrap_or_default();
    match act {
        Act::Close => editor.open = false,
        Act::AddObject => {
            let ty = types.first().cloned().unwrap_or_else(|| "object".into());
            let name = auto_name(editor, &ty);
            editor.objects.push((name, ty));
        }
        Act::RemoveObject(i) => {
            if *i < editor.objects.len() {
                editor.objects.remove(*i);
            }
        }
        Act::CycleType(i) => {
            if let Some(o) = editor.objects.get_mut(*i) {
                o.1 = next_in(&o.1, &types);
            }
        }
        Act::AddFact(goal) => {
            if let Some(d) = &domain {
                if let Some((pred, args)) = d.predicates.first() {
                    let fact = new_fact(&pred.to_lowercase(), args, editor, d);
                    list_mut(editor, *goal).push(fact);
                }
            }
        }
        Act::RemoveFact(goal, i) => {
            let l = list_mut(editor, *goal);
            if *i < l.len() {
                l.remove(*i);
            }
        }
        Act::CyclePred(goal, i) => {
            if let Some(d) = &domain {
                let preds: Vec<String> =
                    d.predicates.iter().map(|(n, _)| n.to_lowercase()).collect();
                let cur = list_mut(editor, *goal)[*i].0.clone();
                let np = next_in(&cur, &preds);
                let args_sig = d
                    .predicates
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(&np))
                    .map(|(_, a)| a.clone())
                    .unwrap_or_default();
                let fact = new_fact(&np, &args_sig, editor, d);
                list_mut(editor, *goal)[*i] = fact;
            }
        }
        Act::CycleArg(goal, fi, slot) => {
            if let Some(d) = &domain {
                let pred = list_mut(editor, *goal)[*fi].0.clone();
                if let Some(arg_ty) = d
                    .predicates
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(&pred))
                    .and_then(|(_, a)| a.get(*slot))
                {
                    let opts = compatible_objects(editor, d, arg_ty);
                    let cur = list_mut(editor, *goal)[*fi].1[*slot].clone();
                    let next = next_in(&cur, &opts);
                    list_mut(editor, *goal)[*fi].1[*slot] = next;
                }
            }
        }
        Act::AddType => {
            let name = auto_name(editor, "type");
            editor.types.push((name, String::new()));
            editor.focus = Some(Focus::TypeName(editor.types.len() - 1));
        }
        Act::RemoveType(i) => {
            if *i < editor.types.len() {
                editor.types.remove(*i);
            }
            editor.focus = None;
        }
        Act::CycleSuper(i) => {
            let names: Vec<String> = std::iter::once(String::new())
                .chain(
                    editor
                        .types
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != *i)
                        .map(|(_, t)| t.0.clone()),
                )
                .collect();
            if let Some(t) = editor.types.get_mut(*i) {
                let cur = t.1.clone();
                t.1 = next_in(&cur, &names);
            }
        }
        Act::AddPred => {
            let name = auto_name(editor, "pred");
            editor.dpreds.push((name, vec![]));
            editor.focus = Some(Focus::PredName(editor.dpreds.len() - 1));
        }
        Act::RemovePred(i) => {
            if *i < editor.dpreds.len() {
                editor.dpreds.remove(*i);
            }
            editor.focus = None;
        }
        Act::AddArg(i) => {
            let ty = editor
                .types
                .first()
                .map(|t| t.0.clone())
                .unwrap_or_else(|| "object".into());
            if let Some(p) = editor.dpreds.get_mut(*i) {
                p.1.push(ty);
            }
        }
        Act::RemoveArg(i) => {
            if let Some(p) = editor.dpreds.get_mut(*i) {
                p.1.pop();
            }
        }
        Act::CycleArgType(i, slot) => {
            let names: Vec<String> = editor.types.iter().map(|t| t.0.clone()).collect();
            if let Some(a) = editor.dpreds.get_mut(*i).and_then(|p| p.1.get_mut(*slot)) {
                *a = next_in(a, &names);
            }
        }
        Act::AddAction => {
            let name = auto_name(editor, "action");
            editor.actions.push(EdAction::Modeled {
                name,
                params: vec![],
                pre: vec![],
                pre_or: false,
                eff: vec![],
                whens: vec![],
            });
            editor.focus = Some(Focus::ActionName(editor.actions.len() - 1));
        }
        Act::RemoveAction(i) => {
            if *i < editor.actions.len() {
                editor.actions.remove(*i);
            }
            editor.focus = None;
        }
        Act::AddParam(i) => {
            let ty = types.first().cloned().unwrap_or_else(|| "object".into());
            if let Some(EdAction::Modeled { params, .. }) = editor.actions.get_mut(*i) {
                let n = format!("?p{}", params.len());
                params.push((n, ty));
            }
        }
        Act::RemoveParam(i) => {
            if let Some(EdAction::Modeled { params, .. }) = editor.actions.get_mut(*i) {
                params.pop();
            }
        }
        Act::CycleParamType(i, k) => {
            if let Some(EdAction::Modeled { params, .. }) = editor.actions.get_mut(*i) {
                if let Some(p) = params.get_mut(*k) {
                    p.1 = next_in(&p.1, &types);
                }
            }
        }
        Act::TogglePreKind(i) => {
            if let Some(EdAction::Modeled { pre_or, .. }) = editor.actions.get_mut(*i) {
                *pre_or = !*pre_or;
            }
        }
        Act::AddWhen(i) => {
            if let Some(EdAction::Modeled { whens, .. }) = editor.actions.get_mut(*i) {
                whens.push((vec![], vec![]));
            }
        }
        Act::RemoveWhen(i, w) => {
            if let Some(EdAction::Modeled { whens, .. }) = editor.actions.get_mut(*i) {
                if *w < whens.len() {
                    whens.remove(*w);
                }
            }
        }
        Act::AddLit(i, loc) => {
            let params = action_params(editor, *i);
            let tys = editor.types.clone();
            let pred0 = editor.dpreds.first().cloned();
            if let (Some((pname, psig)), Some(list)) = (
                pred0,
                editor
                    .actions
                    .get_mut(*i)
                    .and_then(|a| lits_at_mut(a, *loc)),
            ) {
                let args = psig
                    .iter()
                    .map(|aty| first_compatible_param(&params, &tys, aty))
                    .collect();
                list.push((false, pname, args));
            }
        }
        Act::RemoveLit(i, loc, k) => {
            if let Some(list) = editor
                .actions
                .get_mut(*i)
                .and_then(|a| lits_at_mut(a, *loc))
            {
                if *k < list.len() {
                    list.remove(*k);
                }
            }
        }
        Act::ToggleNeg(i, loc, k) => {
            if let Some(list) = editor
                .actions
                .get_mut(*i)
                .and_then(|a| lits_at_mut(a, *loc))
            {
                if let Some(lit) = list.get_mut(*k) {
                    lit.0 = !lit.0;
                }
            }
        }
        Act::CycleLitPred(i, loc, k) => {
            let params = action_params(editor, *i);
            let tys = editor.types.clone();
            let dpreds = editor.dpreds.clone();
            let preds: Vec<String> = dpreds.iter().map(|(n, _)| n.clone()).collect();
            if let Some(list) = editor
                .actions
                .get_mut(*i)
                .and_then(|a| lits_at_mut(a, *loc))
            {
                if let Some(lit) = list.get_mut(*k) {
                    let np = next_in(&lit.1, &preds);
                    let sig = dpreds
                        .iter()
                        .find(|(n, _)| *n == np)
                        .map(|(_, a)| a.clone())
                        .unwrap_or_default();
                    lit.1 = np;
                    lit.2 = sig
                        .iter()
                        .map(|aty| first_compatible_param(&params, &tys, aty))
                        .collect();
                }
            }
        }
        Act::CycleLitArg(i, loc, k, slot) => {
            let params = action_params(editor, *i);
            let tys = editor.types.clone();
            let dpreds = editor.dpreds.clone();
            if let Some(list) = editor
                .actions
                .get_mut(*i)
                .and_then(|a| lits_at_mut(a, *loc))
            {
                if let Some(lit) = list.get_mut(*k) {
                    let arg_ty = dpreds
                        .iter()
                        .find(|(n, _)| *n == lit.1)
                        .and_then(|(_, a)| a.get(*slot))
                        .cloned();
                    if let (Some(aty), Some(cur)) = (arg_ty, lit.2.get(*slot).cloned()) {
                        let opts = compatible_params(&params, &tys, &aty);
                        lit.2[*slot] = next_in(&cur, &opts);
                    }
                }
            }
        }
        Act::SetFocus(f) => editor.focus = Some(*f),
        Act::ToggleMode => {
            editor.mode = if editor.mode == Mode::Problem {
                Mode::Domain
            } else {
                Mode::Problem
            };
            editor.focus = None;
            if editor.mode == Mode::Domain && !editor.dseeded {
                seed_domain(editor, scene);
            }
        }
        Act::Apply => apply_changes(editor, scene),
        Act::Export => export_changes(editor),
    }
}

/// Regenerate PDDL from the editor and reload it. When the domain has been
/// edited, the domain is reloaded first, then the problem against it.
fn apply_changes(editor: &mut Editor, scene: &mut Scene) {
    if editor.dseeded {
        let actions: Vec<String> = editor.actions.iter().map(action_to_pddl).collect();
        let dp = viz::domain_to_pddl(
            &editor.dname,
            &editor.requirements,
            &editor.types,
            &editor.dpreds,
            &actions,
        );
        scene.load_src(&dp);
    }
    let pp = viz::to_pddl(
        &editor.problem_name,
        &editor.dname,
        &editor.objects,
        &editor.init,
        &editor.goal,
    );
    scene.load_src(&pp);
    editor.status = "applied".into();
}

fn export_changes(editor: &mut Editor) {
    let mut wrote = Vec::new();
    if editor.dseeded {
        let actions: Vec<String> = editor.actions.iter().map(action_to_pddl).collect();
        let dp = viz::domain_to_pddl(
            &editor.dname,
            &editor.requirements,
            &editor.types,
            &editor.dpreds,
            &actions,
        );
        let path = format!("/tmp/{}-domain.pddl", editor.dname);
        if std::fs::write(&path, dp).is_ok() {
            wrote.push(path);
        }
    }
    let pp = viz::to_pddl(
        &editor.problem_name,
        &editor.dname,
        &editor.objects,
        &editor.init,
        &editor.goal,
    );
    let path = format!("/tmp/{}.pddl", editor.problem_name);
    if std::fs::write(&path, pp).is_ok() {
        wrote.push(path);
    }
    editor.status = format!("wrote {}", wrote.join(", "));
}

fn list_mut(editor: &mut Editor, goal: bool) -> &mut Vec<(String, Vec<String>)> {
    if goal {
        &mut editor.goal
    } else {
        &mut editor.init
    }
}

fn auto_name(editor: &mut Editor, ty: &str) -> String {
    let c = editor.counters.entry(ty.to_string()).or_insert(0);
    *c += 1;
    format!("{ty}{c}")
}

fn new_fact(pred: &str, sig: &[String], editor: &Editor, domain: &Domain) -> (String, Vec<String>) {
    let args = sig
        .iter()
        .map(|t| {
            compatible_objects(editor, domain, t)
                .first()
                .cloned()
                .unwrap_or_else(|| "?".into())
        })
        .collect();
    (pred.to_string(), args)
}

fn compatible_objects(editor: &Editor, domain: &Domain, arg_ty: &str) -> Vec<String> {
    editor
        .objects
        .iter()
        .filter(|(_, t)| is_subtype(domain, t, arg_ty))
        .map(|(o, _)| o.clone())
        .collect()
}

fn is_subtype(domain: &Domain, ty: &str, of: &str) -> bool {
    if of.eq_ignore_ascii_case("object") {
        return true;
    }
    let mut cur = ty.to_string();
    for _ in 0..64 {
        if cur.eq_ignore_ascii_case(of) {
            return true;
        }
        match domain
            .type_parent
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(&cur))
        {
            Some((_, p)) => cur = p.clone(),
            None => return false,
        }
    }
    false
}

fn next_in(current: &str, list: &[String]) -> String {
    if list.is_empty() {
        return current.to_string();
    }
    match list.iter().position(|x| x == current) {
        Some(i) => list[(i + 1) % list.len()].clone(),
        None => list[0].clone(),
    }
}

// ---- rebuild the bevy_ui tree from Editor ----

pub fn rebuild(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    roots: Query<Entity, With<EditorRoot>>,
) {
    if !editor.dirty {
        return;
    }
    editor.dirty = false;
    for e in &roots {
        commands.entity(e).despawn_recursive();
    }
    if !editor.open {
        return;
    }
    let editor = &*editor;
    commands
        .spawn((
            EditorRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(300.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(3.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.97)),
        ))
        .with_children(|p| {
            row(p, |h| {
                let toggle = match editor.mode {
                    Mode::Problem => "Domain >",
                    Mode::Domain => "< Problem",
                };
                btn(h, toggle, Act::ToggleMode);
                btn(h, "Apply", Act::Apply);
                btn(h, "Export", Act::Export);
                btn(h, "Close", Act::Close);
            });
            match editor.mode {
                Mode::Problem => build_problem(p, editor),
                Mode::Domain => build_domain(p, editor),
            }
            if !editor.status.is_empty() {
                label(p, &editor.status);
            }
        });
}

fn build_problem(p: &mut ChildBuilder, editor: &Editor) {
    label(p, "OBJECTS");
    for (i, (name, ty)) in editor.objects.iter().enumerate() {
        block_card(p, ty, |r| {
            label(r, name);
            btn(r, format!(": {ty}"), Act::CycleType(i));
            btn(r, "x", Act::RemoveObject(i));
        });
    }
    btn(p, "+ object", Act::AddObject);

    zone(p, Zone::Init, |z| {
        label(z, "INIT  (drag :: between zones)");
        for (i, (pred, args)) in editor.init.iter().enumerate() {
            fact_row(z, false, i, pred, args);
        }
        btn(z, "+ fact", Act::AddFact(false));
    });

    zone(p, Zone::Goal, |z| {
        label(z, "GOAL");
        for (i, (pred, args)) in editor.goal.iter().enumerate() {
            fact_row(z, true, i, pred, args);
        }
        btn(z, "+ goal", Act::AddFact(true));
    });
}

/// A drop-zone container (tagged + cursor-tracked) holding a column of blocks.
fn zone(p: &mut ChildBuilder, z: Zone, f: impl FnOnce(&mut ChildBuilder)) {
    p.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(4.0)),
            margin: UiRect::vertical(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.6)),
        z,
        Interaction::default(),
        RelativeCursorPosition::default(),
    ))
    .with_children(f);
}

/// The drag handle at the start of a fact row.
fn grip(r: &mut ChildBuilder, kind: DragKind) {
    r.spawn((
        Node {
            padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.32, 0.32, 0.38)),
        kind,
        Interaction::default(),
        RelativeCursorPosition::default(),
    ))
    .with_children(|g| {
        g.spawn((
            Text::new("::"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ));
    });
}

fn build_domain(p: &mut ChildBuilder, editor: &Editor) {
    let focus = editor.focus;
    row(p, |h| {
        label(h, "domain:");
        btn(
            h,
            name_label(&editor.dname, focus == Some(Focus::DomainName)),
            Act::SetFocus(Focus::DomainName),
        );
    });

    label(p, "TYPES");
    for (i, (name, parent)) in editor.types.iter().enumerate() {
        row(p, |r| {
            btn(
                r,
                name_label(name, focus == Some(Focus::TypeName(i))),
                Act::SetFocus(Focus::TypeName(i)),
            );
            let sup = if parent.is_empty() {
                "- *".to_string()
            } else {
                format!("- {parent}")
            };
            btn(r, sup, Act::CycleSuper(i));
            btn(r, "x", Act::RemoveType(i));
        });
    }
    btn(p, "+ type", Act::AddType);

    label(p, "PREDICATES");
    for (i, (name, args)) in editor.dpreds.iter().enumerate() {
        block_card(p, name, |r| {
            label(r, "(");
            btn(
                r,
                name_label(name, focus == Some(Focus::PredName(i))),
                Act::SetFocus(Focus::PredName(i)),
            );
            for (s, t) in args.iter().enumerate() {
                btn(r, t.clone(), Act::CycleArgType(i, s));
            }
            btn(r, "+arg", Act::AddArg(i));
            if !args.is_empty() {
                btn(r, "-arg", Act::RemoveArg(i));
            }
            label(r, ")");
            btn(r, "x", Act::RemovePred(i));
        });
    }
    btn(p, "+ predicate", Act::AddPred);

    label(p, "ACTIONS");
    for (i, act) in editor.actions.iter().enumerate() {
        match act {
            EdAction::Raw(_) => {
                row(p, |r| {
                    label(r, "(complex action - raw)");
                    btn(r, "x", Act::RemoveAction(i));
                });
            }
            EdAction::Modeled {
                name,
                params,
                pre,
                pre_or,
                eff,
                whens,
            } => {
                row(p, |r| {
                    label(r, "act");
                    btn(
                        r,
                        name_label(name, focus == Some(Focus::ActionName(i))),
                        Act::SetFocus(Focus::ActionName(i)),
                    );
                    btn(r, "x", Act::RemoveAction(i));
                });
                row(p, |r| {
                    label(r, "  params");
                    for (k, (pn, pt)) in params.iter().enumerate() {
                        btn(r, format!("{pn}:{pt}"), Act::CycleParamType(i, k));
                    }
                    btn(r, "+", Act::AddParam(i));
                    if !params.is_empty() {
                        btn(r, "-", Act::RemoveParam(i));
                    }
                });
                row(p, |r| {
                    label(r, "  pre");
                    btn(
                        r,
                        if *pre_or { "any (or)" } else { "all (and)" },
                        Act::TogglePreKind(i),
                    );
                });
                lit_section(p, i, LitLoc::Pre, "", pre);
                lit_section(p, i, LitLoc::Eff, "  eff:", eff);
                for (wi, (cond, weff)) in whens.iter().enumerate() {
                    row(p, |r| {
                        label(r, "  when");
                        btn(r, "x", Act::RemoveWhen(i, wi));
                    });
                    lit_section(p, i, LitLoc::WhenCond(wi), "    if:", cond);
                    lit_section(p, i, LitLoc::WhenEff(wi), "    then:", weff);
                }
                btn(p, "  + when", Act::AddWhen(i));
            }
        }
    }
    btn(p, "+ action", Act::AddAction);
}

fn lit_section(p: &mut ChildBuilder, i: usize, loc: LitLoc, title: &str, lits: &[Lit]) {
    if !title.is_empty() {
        label(p, title);
    }
    for (k, (neg, pred, args)) in lits.iter().enumerate() {
        block_card(p, pred, |r| {
            btn(
                r,
                if *neg { "neg" } else { "pos" },
                Act::ToggleNeg(i, loc, k),
            );
            btn(r, format!("({pred}"), Act::CycleLitPred(i, loc, k));
            for (s, a) in args.iter().enumerate() {
                btn(r, a.clone(), Act::CycleLitArg(i, loc, k, s));
            }
            btn(r, ") x", Act::RemoveLit(i, loc, k));
        });
    }
    btn(p, "  + literal", Act::AddLit(i, loc));
}

/// A name field's label; shows `?` when empty and a trailing `_` cursor when focused.
fn name_label(name: &str, focused: bool) -> String {
    let base = if name.is_empty() { "?" } else { name };
    if focused {
        format!("{base}_")
    } else {
        base.to_string()
    }
}

fn fact_row(p: &mut ChildBuilder, goal: bool, i: usize, pred: &str, args: &[String]) {
    let kind = if goal {
        DragKind::Goal(i)
    } else {
        DragKind::Init(i)
    };
    block_card(p, pred, |r| {
        grip(r, kind);
        btn(r, format!("({pred}"), Act::CyclePred(goal, i));
        for (s, a) in args.iter().enumerate() {
            btn(r, a.clone(), Act::CycleArg(goal, i, s));
        }
        btn(r, ")  x", Act::RemoveFact(goal, i));
    });
}

fn row(p: &mut ChildBuilder, f: impl FnOnce(&mut ChildBuilder)) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(4.0),
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(f);
}

fn label(p: &mut ChildBuilder, text: impl Into<String>) {
    p.spawn((
        Text::new(text.into()),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.82)),
    ));
}

fn btn(p: &mut ChildBuilder, text: impl Into<String>, act: Act) {
    p.spawn((
        Button,
        Node {
            padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.22, 0.22, 0.29)),
        BorderRadius::all(Val::Px(5.0)),
        act,
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(text.into()),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.92, 0.95)),
        ));
    });
}

/// A stable per-key color (fill, accent) so each predicate / type reads as its
/// own kind of block — the Blockly-style visual cue.
fn block_color(key: &str) -> (Color, Color) {
    const P: [([f32; 3], [f32; 3]); 6] = [
        ([0.10, 0.16, 0.27], [0.29, 0.62, 0.93]), // blue
        ([0.09, 0.20, 0.13], [0.16, 0.77, 0.40]), // green
        ([0.25, 0.17, 0.05], [0.96, 0.62, 0.05]), // orange
        ([0.17, 0.12, 0.27], [0.58, 0.40, 0.96]), // purple
        ([0.25, 0.10, 0.18], [0.93, 0.30, 0.62]), // pink
        ([0.05, 0.19, 0.22], [0.05, 0.72, 0.84]), // teal
    ];
    let h = key
        .bytes()
        .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let (f, a) = P[(h as usize) % P.len()];
    (
        Color::srgba(f[0], f[1], f[2], 0.92),
        Color::srgb(a[0], a[1], a[2]),
    )
}

/// A rounded, color-coded block card with a thick left accent edge (Blockly look)
/// wrapping a row of fields. `key` picks the color (predicate or type name).
fn block_card(p: &mut ChildBuilder, key: &str, f: impl FnOnce(&mut ChildBuilder)) {
    let (fill, accent) = block_color(key);
    p.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
            border: UiRect {
                left: Val::Px(4.0),
                top: Val::Px(1.0),
                right: Val::Px(1.0),
                bottom: Val::Px(1.0),
            },
            ..default()
        },
        BackgroundColor(fill),
        BorderColor(accent),
        BorderRadius::all(Val::Px(6.0)),
    ))
    .with_children(f);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

    const DOMAIN: &str = include_str!("../demo/domain.pddl");
    const PROBLEM: &str = include_str!("../demo/problem.pddl");

    fn loaded() -> (Scene, Editor) {
        let mut scene = Scene::default();
        scene.load_src(DOMAIN);
        scene.load_src(PROBLEM);
        let mut ed = Editor::default();
        seed(&mut ed, &scene);
        (scene, ed)
    }

    #[test]
    fn seed_reads_problem() {
        let (_s, ed) = loaded();
        assert_eq!(ed.objects.len(), 8); // 5 locations + truck + 2 crates
        assert!(ed.init.iter().any(|(p, _)| p == "truck-at"));
        assert_eq!(ed.goal.len(), 2);
    }

    #[test]
    fn add_object_and_fact_then_apply_roundtrips() {
        let (mut scene, mut ed) = loaded();
        apply_act(&Act::AddObject, &mut ed, &mut scene);
        assert_eq!(ed.objects.len(), 9);
        let n_init = ed.init.len();
        apply_act(&Act::AddFact(false), &mut ed, &mut scene);
        assert_eq!(ed.init.len(), n_init + 1);

        // Apply regenerates PDDL and re-parses it: the new object survives.
        apply_act(&Act::Apply, &mut ed, &mut scene);
        let prob = scene.problem.as_ref().expect("problem reparsed");
        assert_eq!(prob.objects.len(), 9);
    }

    #[test]
    fn cycle_arg_only_offers_compatible_objects() {
        let (mut scene, mut ed) = loaded();
        // a fresh (pkg-at <crate> <location>) goal fact
        apply_act(&Act::AddFact(true), &mut ed, &mut scene); // first pred = road; cycle to pkg-at
        let gi = ed.goal.len() - 1;
        while ed.goal[gi].0 != "pkg-at" {
            apply_act(&Act::CyclePred(true, gi), &mut ed, &mut scene);
        }
        // slot 0 is a package: cycling must stay within crate1/crate2
        let before = ed.goal[gi].1[0].clone();
        apply_act(&Act::CycleArg(true, gi, 0), &mut ed, &mut scene);
        let after = ed.goal[gi].1[0].clone();
        assert!(["crate1", "crate2"].contains(&before.as_str()));
        assert!(["crate1", "crate2"].contains(&after.as_str()));
    }

    #[test]
    fn seed_domain_reads_domain() {
        let (scene, mut ed) = loaded();
        seed_domain(&mut ed, &scene);
        assert_eq!(ed.dname, "driving");
        assert_eq!(ed.types.len(), 3); // location, truck, package
        assert!(ed.dpreds.iter().any(|(n, _)| n == "road"));
        assert_eq!(ed.actions.len(), 3); // drive, load, unload
        assert!(ed.requirements.contains(":strips"));
    }

    fn domain_pddl(ed: &Editor) -> String {
        let actions: Vec<String> = ed.actions.iter().map(action_to_pddl).collect();
        viz::domain_to_pddl(&ed.dname, &ed.requirements, &ed.types, &ed.dpreds, &actions)
    }

    #[test]
    fn edited_domain_roundtrips_through_parser() {
        let (mut scene, mut ed) = loaded();
        seed_domain(&mut ed, &scene);

        apply_act(&Act::AddType, &mut ed, &mut scene);
        let f = ed.focus.unwrap();
        focused_field_mut(&mut ed, f).unwrap().clear();
        focused_field_mut(&mut ed, f).unwrap().push_str("widget");

        apply_act(&Act::AddPred, &mut ed, &mut scene);
        let f = ed.focus.unwrap();
        focused_field_mut(&mut ed, f).unwrap().clear();
        focused_field_mut(&mut ed, f).unwrap().push_str("shiny");
        apply_act(&Act::AddArg(ed.dpreds.len() - 1), &mut ed, &mut scene);

        let d = ferroplan::parser::parse_domain(&domain_pddl(&ed)).expect("regenerated parses");
        assert!(d.types.iter().any(|t| t.eq_ignore_ascii_case("widget")));
        assert!(d
            .predicates
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("shiny")));
        assert_eq!(d.actions.len(), 3); // modeled + re-emitted
    }

    #[test]
    fn modeled_action_roundtrips() {
        let (scene, mut ed) = loaded();
        seed_domain(&mut ed, &scene);
        assert!(
            ed.actions
                .iter()
                .all(|a| matches!(a, EdAction::Modeled { .. })),
            "flat demo actions should be modeled, not raw"
        );
        let d = ferroplan::parser::parse_domain(&domain_pddl(&ed)).expect("parses");
        let drive = d
            .actions
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case("drive"))
            .expect("drive survives");
        assert_eq!(drive.params.len(), 3); // ?t ?from ?to
    }

    #[test]
    fn add_action_builds_valid_pddl() {
        let (mut scene, mut ed) = loaded();
        seed_domain(&mut ed, &scene);
        apply_act(&Act::AddAction, &mut ed, &mut scene);
        let ai = ed.actions.len() - 1;
        apply_act(&Act::AddParam(ai), &mut ed, &mut scene);
        apply_act(&Act::AddLit(ai, LitLoc::Pre), &mut ed, &mut scene);
        apply_act(&Act::AddLit(ai, LitLoc::Eff), &mut ed, &mut scene);
        let d = ferroplan::parser::parse_domain(&domain_pddl(&ed)).expect("new action parses");
        assert_eq!(d.actions.len(), 4);
    }

    #[test]
    fn or_precondition_and_when_effect_roundtrip() {
        let (mut scene, mut ed) = loaded();
        seed_domain(&mut ed, &scene);
        apply_act(&Act::AddAction, &mut ed, &mut scene);
        let ai = ed.actions.len() - 1;
        apply_act(&Act::AddParam(ai), &mut ed, &mut scene);
        // or-precondition with two literals
        apply_act(&Act::TogglePreKind(ai), &mut ed, &mut scene);
        apply_act(&Act::AddLit(ai, LitLoc::Pre), &mut ed, &mut scene);
        apply_act(&Act::AddLit(ai, LitLoc::Pre), &mut ed, &mut scene);
        // a conditional effect
        apply_act(&Act::AddWhen(ai), &mut ed, &mut scene);
        apply_act(&Act::AddLit(ai, LitLoc::WhenCond(0)), &mut ed, &mut scene);
        apply_act(&Act::AddLit(ai, LitLoc::WhenEff(0)), &mut ed, &mut scene);

        let pddl = domain_pddl(&ed);
        assert!(
            pddl.contains("(or "),
            "precondition should be a disjunction"
        );
        assert!(pddl.contains("(when "), "effect should be conditional");
        let d = ferroplan::parser::parse_domain(&pddl).expect("or/when domain parses");
        assert_eq!(d.actions.len(), 4);
    }

    #[test]
    fn drag_moves_fact_between_zones() {
        let (_s, mut ed) = loaded();
        let (init0, goal0) = (ed.init.len(), ed.goal.len());
        assert!(resolve_drop(DragKind::Init(0), Zone::Goal, &mut ed));
        assert_eq!(ed.init.len(), init0 - 1);
        assert_eq!(ed.goal.len(), goal0 + 1);
        // dropping back on the same zone is a no-op
        assert!(!resolve_drop(DragKind::Goal(0), Zone::Goal, &mut ed));
    }
}
