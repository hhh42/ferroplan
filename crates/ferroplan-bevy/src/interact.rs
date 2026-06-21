//! Pointer interaction: left-click to select a node/mobile, left-drag a node to
//! move it, and a gizmo ring around the current selection.

use bevy::prelude::*;

use crate::scene::{MainCamera, MobileObj, NodeObj, MOBILE_SIZE, NODE_SIZE};

#[derive(Resource, Default)]
pub struct Selected(pub Option<String>);

#[derive(Resource, Default)]
pub struct DragState {
    node: Option<Entity>,
}

fn cursor_world(window: &Window, cam: &Camera, cam_tf: &GlobalTransform) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    cam.viewport_to_world_2d(cam_tf, cursor).ok()
}

#[allow(clippy::too_many_arguments)]
pub fn interact(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    editor: Res<crate::blocks::Editor>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut nodes: Query<(Entity, &NodeObj, &mut Transform)>,
    mobiles: Query<(&MobileObj, &Transform), Without<NodeObj>>,
    mut selected: ResMut<Selected>,
    mut drag: ResMut<DragState>,
) {
    // The editor panel captures the pointer while open.
    if editor.open {
        return;
    }
    let (Ok(window), Ok((cam, cam_tf))) = (windows.get_single(), cam_q.get_single()) else {
        return;
    };
    let Some(world) = cursor_world(window, cam, cam_tf) else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        let mut hit_name = None;
        let mut hit_node = None;
        for (e, n, tf) in &nodes {
            if tf.translation.truncate().distance(world) <= NODE_SIZE * 0.6 {
                hit_name = Some(n.0.clone());
                hit_node = Some(e);
                break;
            }
        }
        if hit_name.is_none() {
            for (m, tf) in &mobiles {
                if tf.translation.truncate().distance(world) <= MOBILE_SIZE * 0.9 {
                    hit_name = Some(m.0.clone());
                    break;
                }
            }
        }
        selected.0 = hit_name;
        drag.node = hit_node;
    }

    if mouse.pressed(MouseButton::Left) {
        if let Some(e) = drag.node {
            if let Ok((_, _, mut tf)) = nodes.get_mut(e) {
                tf.translation.x = world.x;
                tf.translation.y = world.y;
            }
        }
    }
    if mouse.just_released(MouseButton::Left) {
        drag.node = None;
    }
}

/// Draw a ring around the selected node/mobile.
pub fn draw_selection(
    mut gizmos: Gizmos,
    selected: Res<Selected>,
    nodes: Query<(&NodeObj, &Transform)>,
    mobiles: Query<(&MobileObj, &Transform), Without<NodeObj>>,
) {
    let Some(obj) = &selected.0 else {
        return;
    };
    let ring = Color::srgb(1.0, 1.0, 1.0);
    for (n, tf) in &nodes {
        if &n.0 == obj {
            gizmos.circle_2d(tf.translation.truncate(), NODE_SIZE * 0.7, ring);
            return;
        }
    }
    for (m, tf) in &mobiles {
        if &m.0 == obj {
            gizmos.circle_2d(tf.translation.truncate(), MOBILE_SIZE, ring);
            return;
        }
    }
}
