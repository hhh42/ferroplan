//! Problem editor: build/modify a PDDL problem (objects, initial state, goal)
//! and either **apply** it to the canvas or **export** it as a `.pddl` file.
//! The PDDL generator is unit-tested to round-trip through the parser.

use std::collections::BTreeMap;

use ferroplan::types::{Domain, Formula, Problem, Term};

#[derive(Default)]
pub struct Editor {
    pub active: bool,
    problem_name: String,
    objects: Vec<(String, String)>,   // (name, type)
    init: Vec<(String, Vec<String>)>, // (pred, args)
    goal: Vec<(String, Vec<String>)>, // (pred, args)
    seeded: bool,
    // scratch fields for the add-controls
    new_obj: String,
    new_obj_ty: String,
    builder_pred: String,
    builder_args: Vec<String>,
    export_path: String,
    status: String,
}

/// What the editor wants the app to do after a frame.
pub enum Action {
    None,
    /// Apply this generated PDDL problem to the canvas (re-parse + revisualize).
    Apply(String),
}

impl Editor {
    /// Seed the editor from the loaded problem (once), so you edit a copy.
    pub fn seed(&mut self, problem: &Problem) {
        if self.seeded {
            return;
        }
        self.problem_name = problem.name.to_lowercase();
        self.objects = problem
            .objects
            .iter()
            .map(|(o, t)| (o.to_lowercase(), t.to_lowercase()))
            .collect();
        self.init = problem
            .init_atoms
            .iter()
            .map(|(p, a)| {
                (
                    p.to_lowercase(),
                    a.iter().map(|x| x.to_lowercase()).collect(),
                )
            })
            .collect();
        self.goal = goal_atoms(&problem.goal);
        self.seeded = true;
    }

    /// Re-seed from the (newly) loaded problem on next show.
    pub fn reset_seed(&mut self) {
        self.seeded = false;
    }

    pub fn to_pddl(&self, domain_name: &str) -> String {
        to_pddl(
            &self.problem_name,
            domain_name,
            &self.objects,
            &self.init,
            &self.goal,
        )
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, domain: &Domain) -> Action {
        let mut action = Action::None;

        ui.horizontal(|ui| {
            ui.label("name:");
            ui.text_edit_singleline(&mut self.problem_name);
        });

        // --- objects ---
        ui.collapsing("Objects", |ui| {
            let mut remove = None;
            for (i, (o, t)) in self.objects.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.small_button("✕").clicked() {
                        remove = Some(i);
                    }
                    ui.label(format!("{o} : {t}"));
                });
            }
            if let Some(i) = remove {
                self.objects.remove(i);
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_obj)
                        .desired_width(80.0)
                        .hint_text("name"),
                );
                type_combo(ui, "newobjty", &mut self.new_obj_ty, domain);
                if ui.button("+ obj").clicked()
                    && !self.new_obj.trim().is_empty()
                    && !self.new_obj_ty.is_empty()
                {
                    self.objects
                        .push((self.new_obj.trim().to_lowercase(), self.new_obj_ty.clone()));
                    self.new_obj.clear();
                }
            });
        });

        // --- shared atom builder ---
        ui.collapsing("Add fact / goal", |ui| {
            pred_combo(
                ui,
                "predsel",
                &mut self.builder_pred,
                domain,
                &mut self.builder_args,
            );
            let arity = domain
                .predicates
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&self.builder_pred))
                .map(|(_, a)| a.clone())
                .unwrap_or_default();
            self.builder_args.resize(arity.len(), String::new());
            for (i, ty) in arity.iter().enumerate() {
                obj_combo(ui, i, ty, &self.objects, domain, &mut self.builder_args[i]);
            }
            ui.horizontal(|ui| {
                let ready = !self.builder_pred.is_empty()
                    && self.builder_args.iter().all(|a| !a.is_empty());
                if ui.add_enabled(ready, egui::Button::new("→ init")).clicked() {
                    self.init
                        .push((self.builder_pred.to_lowercase(), self.builder_args.clone()));
                }
                if ui.add_enabled(ready, egui::Button::new("→ goal")).clicked() {
                    self.goal
                        .push((self.builder_pred.to_lowercase(), self.builder_args.clone()));
                }
            });
        });

        // --- init / goal lists ---
        atom_list(ui, "Initial state", &mut self.init);
        atom_list(ui, "Goal", &mut self.goal);

        // --- apply / export ---
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Apply to canvas").clicked() {
                action = Action::Apply(self.to_pddl(&domain.name));
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.export_path)
                    .hint_text("export path .pddl")
                    .desired_width(150.0),
            );
            if ui.button("Export").clicked() && !self.export_path.trim().is_empty() {
                let pddl = self.to_pddl(&domain.name);
                self.status = match std::fs::write(self.export_path.trim(), pddl) {
                    Ok(()) => format!("wrote {}", self.export_path.trim()),
                    Err(e) => format!("error: {e}"),
                };
            }
        });
        if !self.status.is_empty() {
            ui.label(egui::RichText::new(&self.status).weak());
        }

        action
    }
}

fn atom_list(ui: &mut egui::Ui, title: &str, atoms: &mut Vec<(String, Vec<String>)>) {
    ui.collapsing(format!("{title} ({})", atoms.len()), |ui| {
        let mut remove = None;
        for (i, (p, a)) in atoms.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
                ui.label(fmt_atom(p, a));
            });
        }
        if let Some(i) = remove {
            atoms.remove(i);
        }
    });
}

fn type_combo(ui: &mut egui::Ui, id: &str, current: &mut String, domain: &Domain) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if current.is_empty() {
            "type"
        } else {
            current.as_str()
        })
        .show_ui(ui, |ui| {
            for t in domain
                .types
                .iter()
                .chain(std::iter::once(&"object".to_string()))
            {
                let lt = t.to_lowercase();
                ui.selectable_value(current, lt.clone(), lt);
            }
        });
}

fn pred_combo(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    domain: &Domain,
    args: &mut Vec<String>,
) {
    let before = current.clone();
    egui::ComboBox::from_id_salt(id)
        .selected_text(if current.is_empty() {
            "predicate"
        } else {
            current.as_str()
        })
        .show_ui(ui, |ui| {
            for (p, _) in &domain.predicates {
                let lp = p.to_lowercase();
                ui.selectable_value(current, lp.clone(), lp);
            }
        });
    if *current != before {
        args.clear(); // new predicate -> reset arg slots
    }
}

fn obj_combo(
    ui: &mut egui::Ui,
    slot: usize,
    arg_ty: &str,
    objects: &[(String, String)],
    domain: &Domain,
    current: &mut String,
) {
    egui::ComboBox::from_id_salt(("arg", slot))
        .selected_text(if current.is_empty() {
            format!("?{}", arg_ty.to_lowercase())
        } else {
            current.clone()
        })
        .show_ui(ui, |ui| {
            for (o, t) in objects {
                if is_subtype(t, arg_ty, domain) {
                    ui.selectable_value(current, o.clone(), o.clone());
                }
            }
        });
}

/// Is `ty` the same as, or a subtype of, `of` (walking the type hierarchy)?
fn is_subtype(ty: &str, of: &str, domain: &Domain) -> bool {
    if of.eq_ignore_ascii_case("object") {
        return true;
    }
    let mut cur = ty.to_string();
    for _ in 0..64 {
        if cur.eq_ignore_ascii_case(of) {
            return true;
        }
        match domain
            .type_parent
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(&cur))
        {
            Some((_, p)) => cur = p.clone(),
            None => return false,
        }
    }
    false
}

fn fmt_atom(pred: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("({})", pred.to_lowercase())
    } else {
        format!(
            "({} {})",
            pred.to_lowercase(),
            args.join(" ").to_lowercase()
        )
    }
}

/// Generate a PDDL problem string. Objects are grouped by type.
pub fn to_pddl(
    name: &str,
    domain_name: &str,
    objects: &[(String, String)],
    init: &[(String, Vec<String>)],
    goal: &[(String, Vec<String>)],
) -> String {
    let mut by_type: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (o, t) in objects {
        by_type
            .entry(t.to_lowercase())
            .or_default()
            .push(o.to_lowercase());
    }
    let objs = by_type
        .iter()
        .map(|(t, os)| format!("{} - {}", os.join(" "), t))
        .collect::<Vec<_>>()
        .join("\n            ");
    let init_s = init
        .iter()
        .map(|(p, a)| fmt_atom(p, a))
        .collect::<Vec<_>>()
        .join(" ");
    let goal_s = goal
        .iter()
        .map(|(p, a)| fmt_atom(p, a))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "(define (problem {})\n  (:domain {})\n  (:objects {})\n  (:init {})\n  (:goal (and {})))\n",
        name.to_lowercase(),
        domain_name.to_lowercase(),
        objs,
        init_s,
        goal_s,
    )
}

/// Flatten the positive ground atoms of a goal formula.
fn goal_atoms(f: &Formula) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    walk(f, &mut out);
    out
}

fn walk(f: &Formula, out: &mut Vec<(String, Vec<String>)>) {
    match f {
        Formula::Atom(p, terms) => {
            let args = terms
                .iter()
                .map(|t| match t {
                    Term::Const(c) => c.to_lowercase(),
                    Term::Var(v) => v.to_lowercase(),
                })
                .collect();
            out.push((p.to_lowercase(), args));
        }
        Formula::And(v) => {
            for x in v {
                walk(x, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroplan::parser::parse_problem;

    #[test]
    fn generated_pddl_round_trips() {
        let objects = vec![
            ("a".into(), "location".into()),
            ("b".into(), "location".into()),
            ("t1".into(), "truck".into()),
        ];
        let init = vec![
            ("at".into(), vec!["t1".into(), "a".into()]),
            ("road".into(), vec!["a".into(), "b".into()]),
        ];
        let goal = vec![("at".into(), vec!["t1".into(), "b".into()])];
        let pddl = to_pddl("p", "logi", &objects, &init, &goal);
        let parsed = parse_problem(&pddl).expect("generated PDDL must parse");
        assert_eq!(parsed.objects.len(), 3);
        assert!(parsed
            .init_atoms
            .iter()
            .any(|(p, a)| p == "AT" && a == &["T1".to_string(), "A".to_string()]));
        // goal is (and (at t1 b))
        assert!(matches!(parsed.goal, Formula::And(_)));
    }
}
