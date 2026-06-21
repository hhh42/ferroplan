//! The inspector: drill into the selected object — its type and supertype chain,
//! the initial facts/fluents it appears in, and the goal atoms it appears in.

use ferroplan::types::Domain;

use crate::model::VizModel;

pub fn show(ui: &mut egui::Ui, domain: Option<&Domain>, model: &VizModel, selected: Option<&str>) {
    ui.heading("Inspector");
    ui.separator();

    let Some(obj) = selected else {
        ui.label(egui::RichText::new("Click an object on the canvas to inspect it.").weak());
        return;
    };

    // type: look it up among nodes/mobiles
    let ty = model
        .nodes
        .iter()
        .find(|n| n.object == obj)
        .map(|n| n.ty.as_str())
        .or_else(|| {
            model
                .mobiles
                .iter()
                .find(|m| m.object == obj)
                .map(|m| m.ty.as_str())
        });

    ui.label(egui::RichText::new(obj.to_lowercase()).strong().size(18.0));
    if let Some(ty) = ty {
        // supertype chain
        let mut chain = vec![ty.to_string()];
        if let Some(d) = domain {
            let mut cur = ty.to_string();
            for _ in 0..64 {
                match d.type_parent.iter().find(|(c, _)| *c == cur) {
                    Some((_, p)) => {
                        chain.push(p.clone());
                        cur = p.clone();
                    }
                    None => break,
                }
            }
        }
        ui.label(format!("type: {}", chain.join(" → ").to_lowercase()));
    }

    // where it is
    if let Some(m) = model.mobiles.iter().find(|m| m.object == obj) {
        if let Some(raw) = &m.at_raw {
            let note = match &m.at {
                Some(node) if node == raw => format!("on: {}", node.to_lowercase()),
                Some(node) => format!("on: {} (via {})", node.to_lowercase(), raw.to_lowercase()),
                None => format!("on: {} (off-graph)", raw.to_lowercase()),
            };
            ui.label(note);
        }
    }

    ui.separator();
    ui.label(egui::RichText::new("Initial facts").strong());
    match model.props_by_object.get(obj) {
        Some(atoms) if !atoms.is_empty() => {
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .id_salt("init")
                .show(ui, |ui| {
                    for a in atoms {
                        ui.label(a);
                    }
                });
        }
        _ => {
            ui.label(egui::RichText::new("(none)").weak());
        }
    }

    ui.separator();
    ui.label(egui::RichText::new("Goal").strong());
    match model.goal_by_object.get(obj) {
        Some(atoms) if !atoms.is_empty() => {
            for a in atoms {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), a);
            }
        }
        _ => {
            ui.label(egui::RichText::new("(not in goal)").weak());
        }
    }
}
