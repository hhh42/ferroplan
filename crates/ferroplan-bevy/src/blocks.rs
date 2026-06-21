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

use ferroplan::types::Domain;
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
    actions_raw: Vec<String>,           // raw (:action …) blocks, preserved
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
    editor.actions_raw = extract_all_blocks(&scene.domain_src, "(:action");
    editor.dseeded = true;
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
        let dp = viz::domain_to_pddl(
            &editor.dname,
            &editor.requirements,
            &editor.types,
            &editor.dpreds,
            &editor.actions_raw,
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
        let dp = viz::domain_to_pddl(
            &editor.dname,
            &editor.requirements,
            &editor.types,
            &editor.dpreds,
            &editor.actions_raw,
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

    label(p, "(actions preserved; editable next)");
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
        let (mut scene, mut ed) = loaded();
        seed_domain(&mut ed, &scene);
        let _ = &mut scene;
        assert_eq!(ed.dname, "driving");
        assert_eq!(ed.types.len(), 3); // location, truck, package
        assert!(ed.dpreds.iter().any(|(n, _)| n == "road"));
        assert_eq!(ed.actions_raw.len(), 3); // drive, load, unload
        assert!(ed.requirements.contains(":strips"));
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

        let pddl = viz::domain_to_pddl(
            &ed.dname,
            &ed.requirements,
            &ed.types,
            &ed.dpreds,
            &ed.actions_raw,
        );
        let d = ferroplan::parser::parse_domain(&pddl).expect("regenerated domain parses");
        assert!(d.types.iter().any(|t| t.eq_ignore_ascii_case("widget")));
        assert!(d
            .predicates
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("shiny")));
        assert_eq!(d.actions.len(), 3); // preserved verbatim
    }
}
