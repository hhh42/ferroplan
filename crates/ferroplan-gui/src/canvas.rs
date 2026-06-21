//! The graph canvas: draw location nodes, connection edges, and mobile objects
//! sitting on their node; drag nodes, pan/zoom the view, click to select.

use std::collections::HashMap;

use egui::{Align2, Color32, FontId, Pos2, Sense, Stroke, Vec2};

use crate::model::VizModel;

const NODE_R: f32 = 24.0;
const MOBILE_R: f32 = 9.0;

pub struct View {
    pub pan: Vec2,
    pub zoom: f32,
    pub needs_fit: bool,
    pub focus: Option<String>,
    dragging: Option<usize>, // node index being dragged
}

impl Default for View {
    fn default() -> Self {
        View {
            pan: Vec2::ZERO,
            zoom: 1.0,
            needs_fit: true,
            focus: None,
            dragging: None,
        }
    }
}

fn to_screen(p: Pos2, center: Pos2, pan: Vec2, zoom: f32) -> Pos2 {
    center + pan + p.to_vec2() * zoom
}

/// Stable pastel color per type name.
fn color_for(ty: &str) -> Color32 {
    let mut h: u32 = 2166136261;
    for b in ty.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    let hue = (h % 360) as f32;
    // simple HSV->RGB at fixed S/V
    let c = 0.45;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (hue / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = 0.35;
    Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

pub fn show(
    ui: &mut egui::Ui,
    model: &mut VizModel,
    view: &mut View,
    selected: &mut Option<String>,
) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    let center = rect.center();

    // node name -> index
    let node_idx: HashMap<String, usize> = model
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.object.clone(), i))
        .collect();

    // fit-to-view on (re)load
    if view.needs_fit && !model.nodes.is_empty() {
        let mut min = Pos2::new(f32::MAX, f32::MAX);
        let mut max = Pos2::new(f32::MIN, f32::MIN);
        for n in &model.nodes {
            min = min.min(n.pos);
            max = max.max(n.pos);
        }
        let extent = (max - min).max_elem().max(1.0);
        view.zoom = (rect.size().min_elem() * 0.8 / extent).clamp(0.05, 4.0);
        let mid = (min.to_vec2() + max.to_vec2()) * 0.5;
        view.pan = -mid * view.zoom;
        view.needs_fit = false;
    }
    // focus an object (center it)
    if let Some(obj) = view.focus.take() {
        let target = node_idx.get(&obj).map(|&i| model.nodes[i].pos).or_else(|| {
            model
                .mobiles
                .iter()
                .find(|m| m.object == obj)
                .and_then(|m| m.at.as_ref())
                .and_then(|n| node_idx.get(n))
                .map(|&i| model.nodes[i].pos)
        });
        if let Some(p) = target {
            view.pan = -p.to_vec2() * view.zoom;
        }
    }

    // --- interaction ---
    let pointer = response.interact_pointer_pos();
    if response.drag_started() {
        view.dragging = pointer.and_then(|p| {
            model.nodes.iter().position(|n| {
                (to_screen(n.pos, center, view.pan, view.zoom) - p).length() <= NODE_R
            })
        });
    }
    if response.dragged() {
        let d = response.drag_delta();
        match view.dragging {
            Some(i) => model.nodes[i].pos += d / view.zoom,
            None => view.pan += d,
        }
    }
    if response.drag_stopped() {
        view.dragging = None;
    }
    if response.clicked() {
        if let Some(p) = pointer {
            *selected = hit_object(model, p, center, view.pan, view.zoom);
        }
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            view.zoom = (view.zoom * (1.0 + scroll * 0.0015)).clamp(0.05, 6.0);
        }
    }

    // --- mobile screen positions (fan around their node; tray for unplaced) ---
    let mut by_node: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut tray: Vec<usize> = Vec::new();
    for (mi, m) in model.mobiles.iter().enumerate() {
        match m.at.as_ref().and_then(|n| node_idx.get(n)) {
            Some(&ni) => by_node.entry(ni).or_default().push(mi),
            None => tray.push(mi),
        }
    }
    let mut mobile_screen: Vec<Pos2> = vec![Pos2::ZERO; model.mobiles.len()];
    for (&ni, ms) in &by_node {
        let nc = to_screen(model.nodes[ni].pos, center, view.pan, view.zoom);
        let k = ms.len().max(1) as f32;
        for (j, &mi) in ms.iter().enumerate() {
            let a = std::f32::consts::TAU * (j as f32) / k - std::f32::consts::FRAC_PI_2;
            mobile_screen[mi] = nc + Vec2::angled(a) * (NODE_R + 16.0);
        }
    }
    for (j, &mi) in tray.iter().enumerate() {
        mobile_screen[mi] = rect.left_bottom() + Vec2::new(24.0 + j as f32 * 46.0, -24.0);
    }

    // --- draw edges ---
    let edge_stroke = Stroke::new(1.5, Color32::from_gray(120));
    for e in &model.edges {
        if let (Some(&a), Some(&b)) = (node_idx.get(&e.a), node_idx.get(&e.b)) {
            painter.line_segment(
                [
                    to_screen(model.nodes[a].pos, center, view.pan, view.zoom),
                    to_screen(model.nodes[b].pos, center, view.pan, view.zoom),
                ],
                edge_stroke,
            );
        }
    }

    // --- draw nodes ---
    let font = FontId::proportional(13.0);
    for n in &model.nodes {
        let c = to_screen(n.pos, center, view.pan, view.zoom);
        let sel = selected.as_deref() == Some(n.object.as_str());
        painter.circle_filled(c, NODE_R, color_for(&n.ty));
        if sel {
            painter.circle_stroke(c, NODE_R + 2.0, Stroke::new(3.0, Color32::WHITE));
        }
        painter.text(
            c + Vec2::new(0.0, NODE_R + 2.0),
            Align2::CENTER_TOP,
            n.object.to_lowercase(),
            font.clone(),
            Color32::from_gray(230),
        );
    }

    // --- draw mobiles ---
    for (mi, m) in model.mobiles.iter().enumerate() {
        let c = mobile_screen[mi];
        let sel = selected.as_deref() == Some(m.object.as_str());
        let col = color_for(&m.ty).gamma_multiply(0.85);
        painter.rect_filled(
            egui::Rect::from_center_size(c, Vec2::splat(MOBILE_R * 2.0)),
            2.0,
            col,
        );
        if sel {
            painter.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::splat(MOBILE_R * 2.0 + 4.0)),
                2.0,
                Stroke::new(2.5, Color32::WHITE),
            );
        }
        painter.text(
            c + Vec2::new(0.0, MOBILE_R + 1.0),
            Align2::CENTER_TOP,
            m.object.to_lowercase(),
            FontId::proportional(11.0),
            Color32::from_gray(210),
        );
    }

    // hint
    painter.text(
        rect.right_bottom() + Vec2::new(-8.0, -6.0),
        Align2::RIGHT_BOTTOM,
        "drag nodes · drag bg to pan · scroll to zoom · click to inspect",
        FontId::proportional(11.0),
        Color32::from_gray(110),
    );
}

fn hit_object(model: &VizModel, p: Pos2, center: Pos2, pan: Vec2, zoom: f32) -> Option<String> {
    // nodes first
    for n in &model.nodes {
        if (to_screen(n.pos, center, pan, zoom) - p).length() <= NODE_R {
            return Some(n.object.clone());
        }
    }
    // then mobiles (recompute positions the same way as draw)
    let node_idx: HashMap<&str, usize> = model
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.object.as_str(), i))
        .collect();
    let mut by_node: HashMap<usize, Vec<usize>> = HashMap::new();
    for (mi, m) in model.mobiles.iter().enumerate() {
        if let Some(&ni) = m.at.as_deref().and_then(|n| node_idx.get(n)) {
            by_node.entry(ni).or_default().push(mi);
        }
    }
    for (&ni, ms) in &by_node {
        let nc = to_screen(model.nodes[ni].pos, center, pan, zoom);
        let k = ms.len().max(1) as f32;
        for (j, &mi) in ms.iter().enumerate() {
            let a = std::f32::consts::TAU * (j as f32) / k - std::f32::consts::FRAC_PI_2;
            let mc = nc + Vec2::angled(a) * (NODE_R + 16.0);
            if (mc - p).length() <= MOBILE_R + 3.0 {
                return Some(model.mobiles[mi].object.clone());
            }
        }
    }
    None
}
