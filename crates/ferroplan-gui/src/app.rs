//! The eframe application shell: load a domain+problem (path fields or by dropping
//! `.pddl` files), lay out the panels (browser / canvas / inspector / timeline),
//! solve off-thread, and animate the plan trace.

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};

use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::types::{Domain, Problem};
use ferroplan::{Mode, Options, StateSnapshot, Step};

use crate::canvas::{self, Anim, View};
use crate::inspect;
use crate::model::VizModel;

struct SolveData {
    mode: Mode,
    steps: Vec<Step>,
    snapshots: Vec<StateSnapshot>,
    metric: Option<f64>,
}
type SolveOut = Result<SolveData, String>;

#[derive(Default)]
pub struct App {
    domain: Option<Domain>,
    problem: Option<Problem>,
    domain_src: String,
    problem_src: String,
    model: VizModel,
    view: View,
    selected: Option<String>,
    object_filter: String,
    domain_path: String,
    problem_path: String,
    domain_err: Option<String>,
    problem_err: Option<String>,

    // plan animation
    plan: Vec<Step>,
    snapshots: Vec<StateSnapshot>,
    timeline: f32,
    playing: bool,
    solve_rx: Option<Receiver<SolveOut>>,
    solve_status: String,
}

impl App {
    fn rebuild(&mut self) {
        if let (Some(d), Some(p)) = (&self.domain, &self.problem) {
            self.model = VizModel::build(d, p);
            self.view.needs_fit = true;
            self.selected = None;
            self.plan.clear();
            self.snapshots.clear();
            self.timeline = 0.0;
            self.playing = false;
            self.solve_status.clear();
        }
    }

    fn set_domain(&mut self, src: &str) {
        match parse_domain(src) {
            Ok(d) => {
                self.domain = Some(d);
                self.domain_src = src.to_string();
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
                self.problem_src = src.to_string();
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
            Ok(src) if as_domain => self.set_domain(&src),
            Ok(src) => self.set_problem(&src),
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

    /// Route a dropped file by content: `(define (problem ...))` vs `(domain ...)`.
    fn drop_file(&mut self, path: &Path) {
        let Ok(src) = std::fs::read_to_string(path) else {
            self.problem_err = Some(format!("cannot read {}", path.display()));
            return;
        };
        let up = src.to_ascii_uppercase();
        let is_problem = match (up.find("(PROBLEM"), up.find("(DOMAIN")) {
            (Some(p), Some(d)) => p < d,
            (Some(_), None) => true,
            _ => false,
        };
        let s = path.display().to_string();
        if is_problem {
            self.problem_path = s;
            self.set_problem(&src);
        } else {
            self.domain_path = s;
            self.set_domain(&src);
        }
    }

    fn solve(&mut self) {
        if self.domain_src.is_empty() || self.problem_src.is_empty() {
            return;
        }
        let (tx, rx) = channel();
        let d = self.domain_src.clone();
        let p = self.problem_src.clone();
        std::thread::spawn(move || {
            let out: SolveOut = match ferroplan::solve(&d, &p, &Options::default()) {
                Ok(sol) => match sol.plan {
                    Some(plan) => {
                        let pairs: Vec<(String, Vec<String>)> = plan
                            .steps
                            .iter()
                            .map(|s| (s.action.clone(), s.args.clone()))
                            .collect();
                        let snapshots = if sol.mode == Mode::Temporal {
                            Vec::new() // overlapping durative actions: no sequential replay
                        } else {
                            ferroplan::trace(&d, &p, &pairs).unwrap_or_default()
                        };
                        Ok(SolveData {
                            mode: sol.mode,
                            steps: plan.steps,
                            snapshots,
                            metric: plan.metric,
                        })
                    }
                    None => Err(if sol.notes.is_empty() {
                        "no plan found".into()
                    } else {
                        sol.notes.join("; ")
                    }),
                },
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(out);
        });
        self.solve_rx = Some(rx);
        self.solve_status = "solving…".into();
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
                });
        }
        if let Some(p) = &self.problem {
            egui::CollapsingHeader::new(format!("problem: {}", p.name.to_lowercase()))
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(format!("{} objects", p.objects.len()));
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

    fn timeline_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let can_solve = !self.domain_src.is_empty()
                && !self.problem_src.is_empty()
                && self.solve_rx.is_none();
            if ui
                .add_enabled(can_solve, egui::Button::new("Solve"))
                .clicked()
            {
                self.solve();
            }
            if !self.solve_status.is_empty() {
                ui.label(&self.solve_status);
            }
        });
        if self.plan.is_empty() {
            return;
        }
        let n = self.plan.len() as f32;
        ui.horizontal(|ui| {
            let label = if self.playing { "Pause" } else { "Play" };
            if ui.button(label).clicked() {
                if self.timeline >= n {
                    self.timeline = 0.0;
                }
                self.playing = !self.playing;
            }
            if ui.button("|<").clicked() {
                self.timeline = (self.timeline.floor() - 1.0).max(0.0);
                self.playing = false;
            }
            if ui.button(">|").clicked() {
                self.timeline = (self.timeline.floor() + 1.0).min(n);
                self.playing = false;
            }
            ui.add(egui::Slider::new(&mut self.timeline, 0.0..=n).show_value(false));
            let k = self.timeline.floor() as usize;
            if k < self.plan.len() {
                let s = &self.plan[k];
                ui.label(format!(
                    "{}/{}: {} {}",
                    k + 1,
                    self.plan.len(),
                    s.action.to_lowercase(),
                    s.args.join(" ").to_lowercase()
                ));
            } else {
                ui.label(format!("{} steps — done", self.plan.len()));
            }
        });
    }

    fn poll_solve(&mut self, ctx: &egui::Context) {
        let done = if let Some(rx) = &self.solve_rx {
            match rx.try_recv() {
                Ok(res) => Some(res),
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint();
                    return;
                }
                Err(_) => Some(Err("solver thread died".into())),
            }
        } else {
            None
        };
        if let Some(res) = done {
            self.solve_rx = None;
            match res {
                Ok(d) => {
                    self.solve_status = format!(
                        "solved: {} steps{}",
                        d.steps.len(),
                        d.metric
                            .map(|m| format!(", metric {m}"))
                            .unwrap_or_default()
                    );
                    if d.mode == Mode::Temporal {
                        self.solve_status.push_str(" (temporal: animation n/a)");
                    }
                    self.plan = d.steps;
                    self.snapshots = d.snapshots;
                    self.timeline = 0.0;
                    self.playing = false;
                }
                Err(e) => {
                    self.solve_status = format!("no plan: {e}");
                    self.plan.clear();
                    self.snapshots.clear();
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // dropped .pddl files
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            if let Some(path) = &f.path {
                self.drop_file(path);
            }
        }

        self.poll_solve(ctx);

        // advance playback
        if self.playing && !self.plan.is_empty() {
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            let n = self.plan.len() as f32;
            self.timeline = (self.timeline + dt * 1.5).min(n);
            if self.timeline >= n {
                self.playing = false;
            }
            ctx.request_repaint();
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

        egui::TopBottomPanel::bottom("timeline").show(ctx, |ui| self.timeline_bar(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.model.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "Load a domain and a problem (or drop .pddl files here), \
                             then Solve to animate the plan.",
                        )
                        .weak(),
                    );
                });
                return;
            }
            // animation frame for the current timeline position
            let maps = if !self.snapshots.is_empty() {
                let count = self.snapshots.len();
                let k = (self.timeline.floor() as usize).min(count - 1);
                let kn = (k + 1).min(count - 1);
                let frac = if kn == k {
                    0.0
                } else {
                    (self.timeline - k as f32).clamp(0.0, 1.0)
                };
                Some((
                    self.model.positions_at(&self.snapshots[k].facts),
                    self.model.positions_at(&self.snapshots[kn].facts),
                    frac,
                ))
            } else {
                None
            };
            let anim = maps.as_ref().map(|(from, to, frac)| Anim {
                from,
                to,
                frac: *frac,
            });
            canvas::show(
                ui,
                &mut self.model,
                &mut self.view,
                &mut self.selected,
                anim.as_ref(),
            );
        });
    }
}
