//! A `bevy_ui` side panel: status + the selected object's facts/goal (inspector).

use bevy::prelude::*;

use crate::anim::Plan;
use crate::interact::Selected;
use crate::scene::Scene;

#[derive(Component)]
pub struct InfoText;

pub fn setup_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                width: Val::Px(340.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::left(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(crate::palette::PANEL_BLUR),
            BorderColor::all(crate::palette::EDGE2),
        ))
        .with_children(|p| {
            // "INSPECTOR" section label
            p.spawn((
                Text::new("INSPECTOR"),
                TextFont {
                    font_size: 10.0_f32.into(),
                    ..default()
                },
                TextColor(crate::palette::FAINT),
            ));
            p.spawn((
                Text::new("drag-drop a domain + problem .pddl here"),
                TextFont {
                    font_size: 14.0_f32.into(),
                    ..default()
                },
                TextColor(crate::palette::INK),
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
                InfoText,
            ));
        });
}

pub fn update_info(
    scene: Res<Scene>,
    selected: Res<Selected>,
    plan: Res<Plan>,
    mut q: Query<&mut Text, With<InfoText>>,
) {
    let Ok(mut text) = q.single_mut() else {
        return;
    };
    let mut s = String::new();
    if scene.status.is_empty() {
        s.push_str("drag-drop a domain + problem .pddl here\n");
    } else {
        s.push_str(&scene.status);
        s.push('\n');
    }
    s.push_str(
        "\nright-drag: pan · scroll: zoom\nclick: inspect · drag a node to move it\n\
         S: solve · Space: play/pause · ←/→: step · R: reset\n",
    );

    // plan / timeline
    if !plan.status.is_empty() {
        s.push_str(&format!("\n{}\n", plan.status));
    }
    if !plan.steps.is_empty() {
        let k = (plan.t.floor() as usize).min(plan.steps.len().saturating_sub(1));
        if (plan.t as usize) < plan.steps.len() {
            let step = &plan.steps[k];
            s.push_str(&format!(
                "step {}/{}: {} {}{}\n",
                k + 1,
                plan.steps.len(),
                step.action.to_lowercase(),
                step.args.join(" ").to_lowercase(),
                if plan.playing { "  [playing]" } else { "" },
            ));
        } else {
            s.push_str(&format!("done ({} steps)\n", plan.steps.len()));
        }
    }

    if let Some(obj) = &selected.0 {
        s.push_str(&format!("\n[{}]\n", obj.to_lowercase()));
        // Live facts from the current snapshot while a plan is loaded; otherwise
        // the initial-state facts.
        if !plan.snapshots.is_empty() {
            let k = (plan.t.floor() as usize).min(plan.snapshots.len() - 1);
            for f in &plan.snapshots[k].facts {
                if fact_mentions(f, obj) {
                    s.push_str(&f.to_lowercase());
                    s.push('\n');
                }
            }
        } else if let Some(ps) = scene.graph.props_by_object.get(obj) {
            for p in ps {
                s.push_str(p);
                s.push('\n');
            }
        }
        if let Some(gs) = scene.graph.goal_by_object.get(obj) {
            s.push_str("\ngoal:\n");
            for g in gs {
                s.push_str(g);
                s.push('\n');
            }
        }
    }
    *text = Text::new(s);
}

/// Does a fact display string (e.g. `(PKG-AT CRATE1 MARKET)`) name `obj` as one
/// of its arguments/predicate tokens?
fn fact_mentions(fact: &str, obj: &str) -> bool {
    fact.split(['(', ')', ' ']).any(|t| t == obj)
}
