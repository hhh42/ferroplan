//! A native, Blockly-style problem editor: build a PDDL problem by assembling
//! object/fact blocks with click fields (no syntax, no text typing). Objects are
//! auto-named; clicking a block field cycles it through the valid choices
//! (type / predicate / type-compatible object). "Apply" regenerates the problem
//! (`viz::to_pddl`) and revisualizes it via `Scene::load_src`; "Export" writes it.
//!
//! `bevy_ui` has no dropdown widget, so fields are buttons that cycle on click;
//! the panel is rebuilt from `Editor` whenever it changes.

use std::collections::HashMap;

use bevy::prelude::*;

use ferroplan::types::Domain;
use ferroplan::viz;

use crate::scene::Scene;

#[derive(Resource, Default)]
pub struct Editor {
    pub open: bool,
    problem_name: String,
    objects: Vec<(String, String)>,   // (name, type)
    init: Vec<(String, Vec<String>)>, // (pred, args)
    goal: Vec<(String, Vec<String>)>, // (pred, args)
    counters: HashMap<String, u32>,
    seeded: bool,
    dirty: bool,
    status: String,
}

#[derive(Component)]
pub struct EditorRoot;

#[derive(Component, Clone)]
pub enum Act {
    AddObject,
    RemoveObject(usize),
    CycleType(usize),
    AddFact(bool), // goal?
    RemoveFact(bool, usize),
    CyclePred(bool, usize),
    CycleArg(bool, usize, usize), // goal?, fact, slot
    Apply,
    Export,
    Close,
}

pub fn toggle_editor(
    keys: Res<ButtonInput<KeyCode>>,
    scene: Res<Scene>,
    mut editor: ResMut<Editor>,
) {
    if keys.just_pressed(KeyCode::KeyE) {
        editor.open = !editor.open;
        if editor.open && !editor.seeded {
            seed(&mut editor, &scene);
        }
        editor.dirty = true;
    }
}

fn seed(editor: &mut Editor, scene: &Scene) {
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
        Act::Apply => {
            let dn = domain.as_ref().map(|d| d.name.clone()).unwrap_or_default();
            let pddl = viz::to_pddl(
                &editor.problem_name,
                &dn,
                &editor.objects,
                &editor.init,
                &editor.goal,
            );
            scene.load_src(&pddl);
            editor.status = "applied".into();
        }
        Act::Export => {
            let dn = domain.as_ref().map(|d| d.name.clone()).unwrap_or_default();
            let pddl = viz::to_pddl(
                &editor.problem_name,
                &dn,
                &editor.objects,
                &editor.init,
                &editor.goal,
            );
            let path = format!("/tmp/{}.pddl", editor.problem_name);
            editor.status = match std::fs::write(&path, pddl) {
                Ok(()) => format!("wrote {path}"),
                Err(e) => format!("error: {e}"),
            };
        }
    }
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
                btn(h, "Apply", Act::Apply);
                btn(h, "Export", Act::Export);
                btn(h, "Close", Act::Close);
            });
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

            if !editor.status.is_empty() {
                label(p, &editor.status);
            }
        });
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
}
