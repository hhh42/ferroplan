//! ferroplan-bevy — a Bevy GUI for visualizing, inspecting, and animating PDDL
//! domains/problems. The graph is the Bevy world (nodes/mobiles as entities,
//! edges as gizmos); logic lives in `ferroplan::viz` + `ferroplan::trace`.

use bevy::prelude::*;

mod anim;
mod interact;
mod scene;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ferroplan — domain visualizer (bevy)".into(),
                resolution: (1280.0, 820.0).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<scene::Scene>()
        .init_resource::<interact::Selected>()
        .init_resource::<interact::DragState>()
        .init_resource::<anim::Plan>()
        .init_resource::<anim::SolveJob>()
        .add_systems(Startup, (scene::setup, ui::setup_ui))
        .add_systems(
            Update,
            (
                scene::handle_drops,
                scene::respawn_graph,
                scene::draw_edges,
                scene::camera_nav,
                interact::interact,
                interact::draw_selection,
                anim::controls,
                anim::poll_solve,
                anim::advance,
                anim::animate,
                ui::update_info,
            ),
        )
        .run();
}
