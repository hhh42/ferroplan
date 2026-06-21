//! ferroplan-bevy — a Bevy GUI for visualizing, inspecting, and animating PDDL
//! domains/problems. The graph is the Bevy world (nodes/mobiles as entities,
//! edges as gizmos); logic lives in `ferroplan::viz` + `ferroplan::trace`.

use bevy::prelude::*;

mod anim;
mod icons;
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
        .add_systems(Startup, (scene::setup, ui::setup_ui, startup_load))
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

/// Optionally load a domain + problem passed on the command line
/// (`ferroplan-bevy domain.pddl problem.pddl`), and pre-select the first mobile.
fn startup_load(mut scene: ResMut<scene::Scene>, mut selected: ResMut<interact::Selected>) {
    for path in std::env::args().skip(1) {
        match std::fs::read_to_string(&path) {
            Ok(src) => scene.load_src(&src),
            Err(e) => eprintln!("cannot read {path}: {e}"),
        }
    }
    if selected.0.is_none() {
        selected.0 = scene.graph.mobiles.first().map(|m| m.object.clone());
    }
}
