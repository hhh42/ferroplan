//! ferroplan-bevy — an unprivileged Bevy visualizer for candidate PDDL plans.
//!
//! The GUI never grants execution authority. Native files and browser handoffs
//! are bounded before parsing; planning runs off the render thread; accepted
//! browser handoff plans are independently validated before animation.

use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;

mod anim;
mod blocks;
mod gantt;
mod icons;
mod interact;
mod palette;
mod scene;
mod transport;
mod ui;
#[cfg(target_arch = "wasm32")]
mod webhandoff;

#[cfg(not(target_arch = "wasm32"))]
const MAX_GUI_FILE_BYTES: usize = 4 * 1024 * 1024;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ferroplan — candidate plan visualizer".into(),
                resolution: (1280, 820).into(),
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#ferroplan-canvas".into()),
                #[cfg(target_arch = "wasm32")]
                fit_canvas_to_parent: true,
                #[cfg(target_arch = "wasm32")]
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(palette::BG))
        .init_resource::<scene::Scene>()
        .init_resource::<interact::Selected>()
        .init_resource::<interact::DragState>()
        .init_resource::<anim::Plan>()
        .init_resource::<anim::SolveJob>()
        .init_resource::<blocks::Editor>()
        .init_resource::<blocks::Drag>()
        .init_resource::<transport::Transport>()
        .init_resource::<gantt::GanttState>()
        .add_systems(
            Startup,
            (
                scene::setup,
                ui::setup_ui,
                transport::setup_transport,
                gantt::setup_gantt,
                startup_load,
            ),
        )
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
                blocks::toggle_editor,
                blocks::text_input,
                blocks::scroll_editor,
                blocks::editor_drag,
                blocks::handle_clicks,
                blocks::rebuild,
            ),
        )
        .add_systems(
            Update,
            (
                transport::transport_visibility,
                transport::rebuild_notches,
                transport::transport_sync,
                transport::transport_input,
                gantt::toggle_gantt,
                gantt::gantt_visibility,
                gantt::rebuild_gantt,
                gantt::gantt_now,
            ),
        )
        .run();
}

fn startup_load(
    mut scene: ResMut<scene::Scene>,
    mut selected: ResMut<interact::Selected>,
    #[cfg(target_arch = "wasm32")] mut plan: ResMut<anim::Plan>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    for path in std::env::args().skip(1) {
        match read_file_bounded(&path) {
            Ok(source) => scene.load_src(&source),
            Err(error) => eprintln!("ferroplan-bevy: refused {path}: {error}"),
        }
    }
    #[cfg(target_arch = "wasm32")]
    if !webhandoff::try_load(&mut scene, &mut plan) {
        scene.load_src(include_str!("../demo/domain.pddl"));
        scene.load_src(include_str!("../demo/problem.pddl"));
        plan.status = "embedded demo · candidate-only".into();
    }
    if selected.0.is_none() {
        selected.0 = scene.graph.mobiles.first().map(|mobile| mobile.object.clone());
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file_bounded(path: &str) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take((MAX_GUI_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_GUI_FILE_BYTES {
        return Err(format!(
            "input exceeds the {MAX_GUI_FILE_BYTES}-byte GUI limit"
        ));
    }
    String::from_utf8(bytes).map_err(|error| format!("input is not UTF-8: {error}"))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn gui_file_limit_is_finite() {
        assert_eq!(MAX_GUI_FILE_BYTES, 4 * 1024 * 1024);
    }
}
