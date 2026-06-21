//! ferroplan-gui — a native (egui/eframe) visualizer & inspector for PDDL
//! domains and problems. Milestone 1: load a domain+problem, render it as an
//! abstract graph (location nodes, connection edges, mobile objects on the
//! graph), drag nodes around, and click any object to inspect its type and the
//! facts it participates in.

mod app;
mod canvas;
mod editor;
mod inspect;
mod model;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_title("ferroplan — domain visualizer"),
        ..Default::default()
    };
    eframe::run_native(
        "ferroplan-gui",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::default()))),
    )
}
