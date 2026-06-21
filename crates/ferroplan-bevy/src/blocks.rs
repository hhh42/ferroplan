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
use bevy::prelude::*;

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

/// An editor action. Flat actions (conjunction of literals) are modeled and
/// editable; anything richer (or/when/forall/numeric) is preserved verbatim.
enum EdAction {
    Modeled {
        name: String,
        params: Vec<(String, String)>, // (?var, type)
        pre: Vec<Lit>,
        eff: Vec<Lit>,
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
    AddLit(usize, bool),
    RemoveLit(usize, bool, usize),
    CycleLitPred(usize, bool, usize),
    CycleLitArg(usize, bool, usize, usize), // action, eff, lit, slot
    ToggleNeg(usize, bool, usize),
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
        (Some(pre), Some(eff)) => EdAction::Modeled {
            name: a.name.to_lowercase(),
            params: a
                .params
                .iter()
                .map(|(n, t)| (param_norm(n), t.to_lowercase()))
                .collect(),
            pre,
            eff,
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

fn flatten_pre(f: &Formula) -> Option<Vec<Lit>> {
    match f {
        Formula::True => Some(vec![]),
        Formula::And(fs) => {
            let mut out = Vec::new();
            for x in fs {
                out.extend(flatten_pre(x)?);
            }
            Some(out)
        }
        Formula::Atom(p, ts) => Some(vec![(
            false,
            p.to_lowercase(),
            ts.iter().map(term_str).collect(),
        )]),
        Formula::Not(b) => match &**b {
            Formula::Atom(p, ts) => Some(vec![(
                true,
                p.to_lowercase(),
                ts.iter().map(term_str).collect(),
            )]),
            _ => None,
        },
        _ => None,
    }
}

fn flatten_eff(e: &Effect) -> Option<Vec<Lit>> {
    match e {
        Effect::And(es) => {
            let mut out = Vec::new();
            for x in es {
                out.extend(flatten_eff(x)?);
            }
            Some(out)
        }
        Effect::Add(p, ts) => Some(vec![(
            false,
            p.to_lowercase(),
            ts.iter().map(term_str).collect(),
        )]),
        Effect::Del(p, ts) => Some(vec![(
            true,
            p.to_lowercase(),
            ts.iter().map(term_str).collect(),
        )]),
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

fn lits_to_pddl(lits: &[Lit]) -> String {
    match lits.len() {
        0 => "(and)".to_string(),
        1 => lit_str(&lits[0]),
        _ => format!(
            "(and {})",
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
            eff,
        } => {
            let ps = params
                .iter()
                .map(|(n, t)| format!("{n} - {t}"))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "(:action {}\n   :parameters ({})\n   :precondition {}\n   :effect {})",
                name,
                ps,
                lits_to_pddl(pre),
                lits_to_pddl(eff)
            )
        }
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
                eff: vec![],
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
        Act::AddLit(i, eff) => {
            let pred0 = editor.dpreds.first().cloned();
            let tys = editor.types.clone();
            if let (
                Some((pname, psig)),
                Some(EdAction::Modeled {
                    params,
                    pre,
                    eff: effv,
                    ..
                }),
            ) = (pred0, editor.actions.get_mut(*i))
            {
                let args = psig
                    .iter()
                    .map(|aty| first_compatible_param(params, &tys, aty))
                    .collect();
                let lit = (false, pname, args);
                if *eff {
                    effv.push(lit);
                } else {
                    pre.push(lit);
                }
            }
        }
        Act::RemoveLit(i, eff, k) => {
            if let Some(EdAction::Modeled { pre, eff: effv, .. }) = editor.actions.get_mut(*i) {
                let l = if *eff { effv } else { pre };
                if *k < l.len() {
                    l.remove(*k);
                }
            }
        }
        Act::ToggleNeg(i, eff, k) => {
            if let Some(EdAction::Modeled { pre, eff: effv, .. }) = editor.actions.get_mut(*i) {
                let l = if *eff { effv } else { pre };
                if let Some(lit) = l.get_mut(*k) {
                    lit.0 = !lit.0;
                }
            }
        }
        Act::CycleLitPred(i, eff, k) => {
            let dpreds = editor.dpreds.clone();
            let tys = editor.types.clone();
            let preds: Vec<String> = dpreds.iter().map(|(n, _)| n.clone()).collect();
            if let Some(EdAction::Modeled {
                params,
                pre,
                eff: effv,
                ..
            }) = editor.actions.get_mut(*i)
            {
                let l = if *eff { effv } else { pre };
                if let Some(lit) = l.get_mut(*k) {
                    let np = next_in(&lit.1, &preds);
                    let sig = dpreds
                        .iter()
                        .find(|(n, _)| *n == np)
                        .map(|(_, a)| a.clone())
                        .unwrap_or_default();
                    lit.1 = np;
                    lit.2 = sig
                        .iter()
                        .map(|aty| first_compatible_param(params, &tys, aty))
                        .collect();
                }
            }
        }
        Act::CycleLitArg(i, eff, k, slot) => {
            let dpreds = editor.dpreds.clone();
            let tys = editor.types.clone();
            if let Some(EdAction::Modeled {
                params,
                pre,
                eff: effv,
                ..
            }) = editor.actions.get_mut(*i)
            {
                let l = if *eff { effv } else { pre };
                if let Some(lit) = l.get_mut(*k) {
                    let arg_ty = dpreds
                        .iter()
                        .find(|(n, _)| *n == lit.1)
                        .and_then(|(_, a)| a.get(*slot))
                        .cloned();
                    if let (Some(aty), Some(cur)) = (arg_ty, lit.2.get(*slot).cloned()) {
                        let opts = compatible_params(params, &tys, &aty);
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
                ..default()
            },
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
        row(p, |r| {
            label(r, name);
            btn(r, format!(": {ty}"), Act::CycleType(i));
            btn(r, "x", Act::RemoveObject(i));
        });
    }
    btn(p, "+ object", Act::AddObject);

    label(p, "INIT");
    for (i, (pred, args)) in editor.init.iter().enumerate() {
        fact_row(p, false, i, pred, args);
    }
    btn(p, "+ fact", Act::AddFact(false));

    label(p, "GOAL");
    for (i, (pred, args)) in editor.goal.iter().enumerate() {
        fact_row(p, true, i, pred, args);
    }
    btn(p, "+ goal", Act::AddFact(true));
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
        row(p, |r| {
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
                eff,
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
                lit_section(p, i, false, "  pre:", pre);
                lit_section(p, i, true, "  eff:", eff);
            }
        }
    }
    btn(p, "+ action", Act::AddAction);
}

fn lit_section(p: &mut ChildBuilder, i: usize, eff: bool, title: &str, lits: &[Lit]) {
    label(p, title);
    for (k, (neg, pred, args)) in lits.iter().enumerate() {
        row(p, |r| {
            btn(
                r,
                if *neg { "neg" } else { "pos" },
                Act::ToggleNeg(i, eff, k),
            );
            btn(r, format!("({pred}"), Act::CycleLitPred(i, eff, k));
            for (s, a) in args.iter().enumerate() {
                btn(r, a.clone(), Act::CycleLitArg(i, eff, k, s));
            }
            btn(r, ") x", Act::RemoveLit(i, eff, k));
        });
    }
    btn(p, "  + literal", Act::AddLit(i, eff));
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
    row(p, |r| {
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
            padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.26)),
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
        apply_act(&Act::AddLit(ai, false), &mut ed, &mut scene);
        apply_act(&Act::AddLit(ai, true), &mut ed, &mut scene);
        let d = ferroplan::parser::parse_domain(&domain_pddl(&ed)).expect("new action parses");
        assert_eq!(d.actions.len(), 4);
    }
}
