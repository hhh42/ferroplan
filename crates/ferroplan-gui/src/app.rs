//! The eframe application shell: load a domain+problem (path fields or by dropping
//! `.pddl` files onto the window), lay out the three panels (browser / canvas /
//! inspector), and hold the shared selection + view state.

use std::path::Path;

use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::types::{Domain, Problem};

use crate::canvas::{self, View};
use crate::inspect;
use crate::model::VizModel;

#[derive(Default)]
pub struct App {
    domain: Option<Domain>,
    problem: Option<Problem>,
    model: VizModel,
    view: View,
    selected: Option<String>,
    object_filter: String,
    domain_path: String,
    problem_path: String,
    domain_err: Option<String>,
    problem_err: Option<String>,
}

impl App {
    fn rebuild(&mut self) {
        if let (Some(d), Some(p)) = (&self.domain, &self.problem) {
            self.model = VizModel::build(d, p);
            self.view.needs_fit = true;
            self.selected = None;
        }
    }

    fn set_domain(&mut self, src: &str) {
        match parse_domain(src) {
            Ok(d) => {
                self.domain = Some(d);
                self.domain_err = None;
                self.rebuild();
            }
            Err(e) => {
                self.domain_err = Some(e.to_string());
                self.domain = None;
            }
        }
    }

    fn set_problem(&mut self, src: &str) {
        match parse_problem(src) {
            Ok(p) => {
                self.problem = Some(p);
                self.problem_err = None;
                self.rebuild();
            }
            Err(e) => {
                self.problem_err = Some(e.to_string());
                self.problem = None;
            }
        }
    }

    fn load_path(&mut self, path: &str, as_domain: bool) {
        match std::fs::read_to_string(path) {
            Ok(src) => {
                if as_domain {
                    self.set_domain(&src);
                } else {
                    self.set_problem(&src);
                }
            }
            Err(e) => {
                let msg = Some(format!("{path}: {e}"));
                if as_domain {
                    self.domain_err = msg;
                } else {
                    self.problem_err = msg;
                }
            }
        }
    }

    /// Route a dropped file by content: a `(define (problem ...))` is a problem,
    /// a `(define (domain ...))` is a domain.
    fn drop_file(&mut self, path: &Path, contents: Option<&str>) {
        let owned;
        let src = match contents {
            Some(s) => s,
            None => match std::fs::read_to_string(path) {
                Ok(s) => {
                    owned = s;
                    &owned
                }
                Err(e) => {
                    self.problem_err = Some(format!("{}: {e}", path.display()));
                    return;
                }
            },
        };
        let up = src.to_ascii_uppercase();
        let p_at = up.find("(PROBLEM");
        let d_at = up.find("(DOMAIN");
        let is_problem = match (p_at, d_at) {
            (Some(p), Some(d)) => p < d,
            (Some(_), None) => true,
            _ => false,
        };
        let path_str = path.display().to_string();
        if is_problem {
            self.problem_path = path_str;
            self.set_problem(src);
        } else {
            self.domain_path = path_str;
            self.set_domain(src);
        }
    }

    fn browser(&mut self, ui: &mut egui::Ui) {
        ui.heading("ferroplan");
        ui.label(egui::RichText::new("domain visualizer").weak());
        ui.separator();

        ui.label("Domain");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.domain_path)
                    .hint_text("path to domain.pddl")
                    .desired_width(150.0),
            );
            if ui.button("Load").clicked() {
                let p = self.domain_path.clone();
                self.load_path(&p, true);
            }
        });
        if let Some(e) = &self.domain_err {
            ui.colored_label(egui::Color32::LIGHT_RED, e);
        }
        ui.label("Problem");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.problem_path)
                    .hint_text("path to problem.pddl")
                    .desired_width(150.0),
            );
            if ui.button("Load").clicked() {
                let p = self.problem_path.clone();
                self.load_path(&p, false);
            }
        });
        if let Some(e) = &self.problem_err {
            ui.colored_label(egui::Color32::LIGHT_RED, e);
        }
        ui.label(egui::RichText::new("…or drop .pddl files on the window").weak());
        ui.separator();

        if let Some(d) = &self.domain {
            egui::CollapsingHeader::new(format!("domain: {}", d.name.to_lowercase()))
                .default_open(true)
                .show(ui, |ui| {
                    let dynamic = crate::model::dynamic_predicates(d).len();
                    ui.label(format!("{} types", d.types.len()));
                    ui.label(format!(
                        "{} predicates ({} dynamic / {} static)",
                        d.predicates.len(),
                        dynamic,
                        d.predicates.len().saturating_sub(dynamic),
                    ));
                    ui.label(format!("{} functions", d.functions.len()));
                    ui.label(format!(
                        "{} actions ({} durative)",
                        d.actions.len(),
                        d.durative_actions.len()
                    ));
                    if !d.requirements.is_empty() {
                        ui.label(
                            egui::RichText::new(d.requirements.join(" ").to_lowercase()).weak(),
                        );
                    }
                });
        }
        if let Some(p) = &self.problem {
            egui::CollapsingHeader::new(format!("problem: {}", p.name.to_lowercase()))
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(format!("{} objects", p.objects.len()));
                    ui.label(format!("{} init facts", p.init_atoms.len()));
                    ui.label(format!(
                        "{} nodes / {} mobiles",
                        self.model.nodes.len(),
                        self.model.mobiles.len()
                    ));
                });
        }

        if self.domain.is_some() && self.problem.is_some() {
            ui.separator();
            ui.label("Objects");
            ui.add(
                egui::TextEdit::singleline(&mut self.object_filter)
                    .hint_text("filter…")
                    .desired_width(f32::INFINITY),
            );
            let filter = self.object_filter.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut names: Vec<&str> = self
                    .model
                    .nodes
                    .iter()
                    .map(|n| n.object.as_str())
                    .chain(self.model.mobiles.iter().map(|m| m.object.as_str()))
                    .collect();
                names.sort();
                for name in names {
                    if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                        continue;
                    }
                    let sel = self.selected.as_deref() == Some(name);
                    if ui.selectable_label(sel, name.to_lowercase()).clicked() {
                        self.selected = Some(name.to_string());
                        self.view.focus = Some(name.to_string());
                    }
                }
            });
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // handle dropped .pddl files
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            if let Some(path) = &f.path {
                let contents = f.bytes.as_ref().and_then(|b| std::str::from_utf8(b).ok());
                self.drop_file(path, contents);
            }
        }

        egui::SidePanel::left("browser")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| self.browser(ui));

        egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                inspect::show(
                    ui,
                    self.domain.as_ref(),
                    &self.model,
                    self.selected.as_deref(),
                );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.model.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "Load a domain and a problem (or drop .pddl files here).\n\
                             Location-typed objects become nodes; everything else \
                             sits on the graph.",
                        )
                        .weak(),
                    );
                });
            } else {
                canvas::show(ui, &mut self.model, &mut self.view, &mut self.selected);
            }
        });
    }
}
