//! The graph as a Bevy world: load a domain+problem (drag-drop), build the
//! `VizGraph`, spawn nodes/mobiles as entities, draw edges with gizmos, and
//! navigate with the camera. Interaction (drag/select), the inspector, and plan
//! animation are layered on in later stages.

use std::collections::HashMap;

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::types::{Domain, Problem};
use ferroplan::viz::VizGraph;

pub const NODE_SIZE: f32 = 44.0;
pub const MOBILE_SIZE: f32 = 18.0;
const GOLDEN: f32 = 2.399_963_2;

/// The loaded domain/problem + derived graph. `dirty` triggers a respawn.
#[derive(Resource, Default)]
pub struct Scene {
    pub domain: Option<Domain>,
    pub problem: Option<Problem>,
    pub domain_src: String,
    pub problem_src: String,
    pub graph: VizGraph,
    pub dirty: bool,
    pub status: String,
}

impl Scene {
    fn rebuild(&mut self) {
        if let (Some(d), Some(p)) = (&self.domain, &self.problem) {
            self.graph = VizGraph::build(d, p);
            self.dirty = true;
            self.status = format!(
                "{}: {} nodes, {} mobiles",
                p.name.to_lowercase(),
                self.graph.nodes.len(),
                self.graph.mobiles.len()
            );
        }
    }

    fn load_src(&mut self, src: &str) {
        let up = src.to_ascii_uppercase();
        let is_problem = match (up.find("(PROBLEM"), up.find("(DOMAIN")) {
            (Some(p), Some(d)) => p < d,
            (Some(_), None) => true,
            _ => false,
        };
        if is_problem {
            match parse_problem(src) {
                Ok(p) => {
                    self.problem = Some(p);
                    self.problem_src = src.to_string();
                    self.rebuild();
                }
                Err(e) => self.status = format!("problem parse error: {e}"),
            }
        } else {
            match parse_domain(src) {
                Ok(d) => {
                    self.domain = Some(d);
                    self.domain_src = src.to_string();
                    self.rebuild();
                }
                Err(e) => self.status = format!("domain parse error: {e}"),
            }
        }
    }
}

#[derive(Component)]
pub struct GraphItem;

#[derive(Component)]
pub struct NodeObj(pub String);

#[derive(Component)]
pub struct MobileObj(pub String);

#[derive(Component)]
pub struct MainCamera;

pub fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

/// Read dropped `.pddl` files and load them (content-routed domain vs problem).
pub fn handle_drops(mut drops: EventReader<FileDragAndDrop>, mut scene: ResMut<Scene>) {
    for ev in drops.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = ev {
            match std::fs::read_to_string(path_buf) {
                Ok(src) => scene.load_src(&src),
                Err(e) => scene.status = format!("cannot read {}: {e}", path_buf.display()),
            }
        }
    }
}

fn color_for(ty: &str) -> Color {
    let mut h: u32 = 2166136261;
    for b in ty.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    let hue = (h % 360) as f32 / 360.0;
    Color::hsl(hue * 360.0, 0.6, 0.6)
}

/// Despawn + respawn all graph entities when the scene changes. Nodes are laid
/// out on a circle; mobiles sit (fanned) on their node.
pub fn respawn_graph(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    existing: Query<Entity, With<GraphItem>>,
) {
    if !scene.dirty {
        return;
    }
    scene.dirty = false;
    for e in &existing {
        commands.entity(e).despawn_recursive();
    }

    let g = &scene.graph;
    let n = g.nodes.len().max(1) as f32;
    let radius = (40.0 * n).max(200.0);
    let mut node_pos: HashMap<String, Vec2> = HashMap::new();
    for (i, node) in g.nodes.iter().enumerate() {
        let a = std::f32::consts::TAU * (i as f32) / n;
        let pos = Vec2::new(radius * a.cos(), radius * a.sin());
        node_pos.insert(node.object.clone(), pos);
        commands.spawn((
            GraphItem,
            NodeObj(node.object.clone()),
            Sprite {
                color: color_for(&node.ty),
                custom_size: Some(Vec2::splat(NODE_SIZE)),
                ..default()
            },
            Transform::from_translation(pos.extend(0.0)),
        ));
        commands.spawn((
            GraphItem,
            Text2d::new(node.object.to_lowercase()),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Transform::from_translation((pos + Vec2::new(0.0, -NODE_SIZE)).extend(1.0)),
        ));
    }

    for (mi, m) in g.mobiles.iter().enumerate() {
        let base =
            m.at.as_ref()
                .and_then(|name| node_pos.get(name).copied())
                .unwrap_or_else(|| Vec2::new(-radius - 120.0, radius - mi as f32 * 40.0));
        let off = Vec2::from_angle(mi as f32 * GOLDEN) * (NODE_SIZE * 0.7);
        let pos = base + off;
        commands.spawn((
            GraphItem,
            MobileObj(m.object.clone()),
            Sprite {
                color: color_for(&m.ty),
                custom_size: Some(Vec2::splat(MOBILE_SIZE)),
                ..default()
            },
            Transform::from_translation(pos.extend(2.0)),
        ));
        commands.spawn((
            GraphItem,
            Text2d::new(m.object.to_lowercase()),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.85)),
            Transform::from_translation((pos + Vec2::new(0.0, -MOBILE_SIZE)).extend(3.0)),
        ));
    }
}

/// Draw connection edges between node entities each frame.
pub fn draw_edges(mut gizmos: Gizmos, scene: Res<Scene>, nodes: Query<(&NodeObj, &Transform)>) {
    let pos: HashMap<&str, Vec2> = nodes
        .iter()
        .map(|(n, t)| (n.0.as_str(), t.translation.truncate()))
        .collect();
    for e in &scene.graph.edges {
        if let (Some(&a), Some(&b)) = (pos.get(e.a.as_str()), pos.get(e.b.as_str())) {
            gizmos.line_2d(a, b, Color::srgb(0.4, 0.4, 0.45));
        }
    }
}

/// Camera navigation: right-drag to pan, scroll to zoom.
pub fn camera_nav(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut wheel: EventReader<MouseWheel>,
    mut cam: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
) {
    let Ok((mut tf, mut proj)) = cam.get_single_mut() else {
        return;
    };
    if mouse.pressed(MouseButton::Right) {
        for m in motion.read() {
            tf.translation.x -= m.delta.x * proj.scale;
            tf.translation.y += m.delta.y * proj.scale;
        }
    } else {
        motion.clear();
    }
    for ev in wheel.read() {
        let step = match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y * 0.02,
        };
        proj.scale = (proj.scale * (1.0 - step * 0.1)).clamp(0.1, 10.0);
    }
}
