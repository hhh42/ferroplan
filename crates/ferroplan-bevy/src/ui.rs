//! A `bevy_ui` side panel: status + the selected object's facts/goal (inspector).

use bevy::prelude::*;

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
                width: Val::Px(320.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.07, 0.09, 0.9)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("drag-drop a domain + problem .pddl here"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.92)),
                InfoText,
            ));
        });
}

pub fn update_info(
    scene: Res<Scene>,
    selected: Res<Selected>,
    mut q: Query<&mut Text, With<InfoText>>,
) {
    let Ok(mut text) = q.get_single_mut() else {
        return;
    };
    let mut s = String::new();
    if scene.status.is_empty() {
        s.push_str("drag-drop a domain + problem .pddl here\n");
    } else {
        s.push_str(&scene.status);
        s.push('\n');
    }
    s.push_str("\nright-drag: pan · scroll: zoom\nclick: inspect · drag a node to move it\n");

    if let Some(obj) = &selected.0 {
        s.push_str(&format!("\n[{}]\n", obj.to_lowercase()));
        if let Some(ps) = scene.graph.props_by_object.get(obj) {
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
