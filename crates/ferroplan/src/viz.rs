//! Visualization model: derive an abstract graph from a parsed domain + problem,
//! plus helpers a GUI needs (per-snapshot positions, PDDL generation). This is
//! pure, view-agnostic logic — no GUI/layout types — so any front-end (Bevy, egui,
//! web) can consume it.
//!
//! Predicates are classified by structure (arity + argument types), with name
//! heuristics as a fallback:
//!   - **edge**: binary over two objects of the same "location" type
//!     (`(road ?a ?b)`); the shared type is a *location*.
//!   - **position**: binary placing a mobile onto a location (`(at ?truck ?loc)`);
//!     arg0's type is *mobile* and wins ties over being a location.
//!   - **property**: everything else (shown in an inspector).
//!
//! Location-typed objects become **nodes**; the rest become **mobiles** resolved
//! (transitively) onto the node they sit on.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::types::{Domain, Effect, Formula, Problem, Term};

const EDGE_NAMES: &[&str] = &[
    "ROAD",
    "LINK",
    "PATH",
    "CONNECTED",
    "ADJACENT",
    "NEXT",
    "EDGE",
    "CONN",
    "CONNECTS",
    "ROUTE",
    "CAN-MOVE",
    "CAN-TRAVERSE",
    "VISIBLE",
];
const POSITION_NAMES: &[&str] = &["AT", "IN", "ON", "LOCATED", "INSIDE", "AT-ROBBY", "HOLDING"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PredKind {
    Edge,
    Position,
    Property,
}

#[derive(Clone, Debug)]
pub struct VizNode {
    pub object: String,
    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct VizEdge {
    pub a: String,
    pub b: String,
    pub pred: String,
}

#[derive(Clone, Debug)]
pub struct VizMobile {
    pub object: String,
    pub ty: String,
    /// The node this object sits on (resolved transitively); None = off-graph.
    pub at: Option<String>,
    /// Raw position target (may be another mobile, e.g. a package in a truck).
    pub at_raw: Option<String>,
}

/// The abstract graph for a domain+problem. Layout (positions) is the view's job.
#[derive(Default, Clone, Debug)]
pub struct VizGraph {
    pub nodes: Vec<VizNode>,
    pub edges: Vec<VizEdge>,
    pub mobiles: Vec<VizMobile>,
    pub props_by_object: BTreeMap<String, Vec<String>>,
    pub goal_by_object: BTreeMap<String, Vec<String>>,
    pub pred_kind: BTreeMap<String, PredKind>,
    pub location_types: BTreeSet<String>,
}

impl VizGraph {
    pub fn build(domain: &Domain, problem: &Problem) -> Self {
        let obj_ty: HashMap<&str, &str> = problem
            .objects
            .iter()
            .chain(domain.constants.iter())
            .map(|(o, t)| (o.as_str(), t.as_str()))
            .collect();

        // 1. classify predicates by signature
        let mut pred_kind: BTreeMap<String, PredKind> = BTreeMap::new();
        let mut location_types: BTreeSet<String> = BTreeSet::new();
        let mut mobile_types: BTreeSet<String> = BTreeSet::new();
        for (name, args) in &domain.predicates {
            let kind = if args.len() == 2 {
                let edgey = EDGE_NAMES.contains(&name.as_str())
                    || (args[0] == args[1] && !POSITION_NAMES.contains(&name.as_str()));
                if edgey {
                    location_types.insert(args[0].clone());
                    location_types.insert(args[1].clone());
                    PredKind::Edge
                } else {
                    mobile_types.insert(args[0].clone());
                    location_types.insert(args[1].clone());
                    PredKind::Position
                }
            } else {
                PredKind::Property
            };
            pred_kind.insert(name.clone(), kind);
        }
        for m in &mobile_types {
            location_types.remove(m);
        }

        let parent: HashMap<&str, &str> = domain
            .type_parent
            .iter()
            .map(|(c, p)| (c.as_str(), p.as_str()))
            .collect();
        let is_location = |ty: &str| -> bool {
            let mut cur = ty;
            for _ in 0..64 {
                if location_types.contains(cur) {
                    return true;
                }
                match parent.get(cur) {
                    Some(p) => cur = p,
                    None => break,
                }
            }
            false
        };

        // 2. nodes (location-typed objects)
        let mut node_objs: Vec<(&str, &str)> = obj_ty
            .iter()
            .filter(|(_, t)| is_location(t))
            .map(|(o, t)| (*o, *t))
            .collect();
        node_objs.sort();
        let nodes: Vec<VizNode> = node_objs
            .iter()
            .map(|(o, t)| VizNode {
                object: (*o).to_string(),
                ty: (*t).to_string(),
            })
            .collect();
        let node_set: BTreeSet<&str> = nodes.iter().map(|x| x.object.as_str()).collect();

        // 3. edges + position targets + per-object properties (from init)
        let mut edges = Vec::new();
        let mut pos_target: HashMap<String, String> = HashMap::new();
        let mut props: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (pred, args) in &problem.init_atoms {
            let atom = fmt_atom(pred, args);
            for a in args {
                props.entry(a.clone()).or_default().push(atom.clone());
            }
            match pred_kind.get(pred) {
                Some(PredKind::Edge) if args.len() == 2 => {
                    if node_set.contains(args[0].as_str()) && node_set.contains(args[1].as_str()) {
                        edges.push(VizEdge {
                            a: args[0].clone(),
                            b: args[1].clone(),
                            pred: pred.clone(),
                        });
                    }
                }
                Some(PredKind::Position) if args.len() == 2 => {
                    pos_target.entry(args[0].clone()).or_insert(args[1].clone());
                }
                _ => {}
            }
        }
        for ((f, args), v) in &problem.init_fluents {
            let s = format!("{} = {}", fmt_atom(f, args), trim_f(*v));
            for a in args {
                props.entry(a.clone()).or_default().push(s.clone());
            }
        }

        // 4. mobiles (non-node objects), resolved to a node transitively
        let resolve = |start: &str| -> Option<String> {
            let mut cur = start.to_string();
            for _ in 0..64 {
                if node_set.contains(cur.as_str()) {
                    return Some(cur);
                }
                match pos_target.get(&cur) {
                    Some(n) => cur = n.clone(),
                    None => return None,
                }
            }
            None
        };
        let mut mobiles: Vec<VizMobile> = obj_ty
            .iter()
            .filter(|(o, _)| !node_set.contains(*o))
            .map(|(o, t)| {
                let at_raw = pos_target.get(*o).cloned();
                let at = at_raw.as_deref().and_then(resolve);
                VizMobile {
                    object: (*o).to_string(),
                    ty: (*t).to_string(),
                    at,
                    at_raw,
                }
            })
            .collect();
        mobiles.sort_by(|a, b| a.object.cmp(&b.object));

        // 5. goal atoms per object
        let mut goal_by_object: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut goal_atoms_v = Vec::new();
        collect_atoms(&problem.goal, false, &mut goal_atoms_v);
        for (pred, args, neg) in goal_atoms_v {
            let s = if neg {
                format!("(not {})", fmt_atom(&pred, &args))
            } else {
                fmt_atom(&pred, &args)
            };
            for a in &args {
                goal_by_object.entry(a.clone()).or_default().push(s.clone());
            }
        }

        VizGraph {
            nodes,
            edges,
            mobiles,
            props_by_object: props,
            goal_by_object,
            pred_kind,
            location_types,
        }
    }

    /// Map each mobile to the node it sits on given a snapshot's true facts
    /// (display strings like `(AT T1 A)`); resolves containment transitively.
    pub fn positions_at(&self, facts: &[String]) -> HashMap<String, Option<String>> {
        let node_set: BTreeSet<&str> = self.nodes.iter().map(|n| n.object.as_str()).collect();
        let mut pos_target: HashMap<String, String> = HashMap::new();
        for f in facts {
            let inner = f.trim().trim_start_matches('(').trim_end_matches(')');
            let mut it = inner.split_whitespace();
            let Some(pred) = it.next() else { continue };
            if self.pred_kind.get(&pred.to_uppercase()) == Some(&PredKind::Position) {
                if let (Some(m), Some(p)) = (it.next(), it.next()) {
                    pos_target
                        .entry(m.to_uppercase())
                        .or_insert_with(|| p.to_uppercase());
                }
            }
        }
        let mut out = HashMap::new();
        for m in &self.mobiles {
            let mut cur = m.object.clone();
            let mut node = None;
            for _ in 0..64 {
                if node_set.contains(cur.as_str()) {
                    node = Some(cur.clone());
                    break;
                }
                match pos_target.get(&cur) {
                    Some(n) => cur = n.clone(),
                    None => break,
                }
            }
            out.insert(m.object.clone(), node);
        }
        out
    }
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

fn trim_f(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn collect_atoms(f: &Formula, neg: bool, out: &mut Vec<(String, Vec<String>, bool)>) {
    match f {
        Formula::Atom(p, terms) => {
            let args = terms
                .iter()
                .map(|t| match t {
                    Term::Const(c) => c.clone(),
                    Term::Var(v) => v.clone(),
                })
                .collect();
            out.push((p.clone(), args, neg));
        }
        Formula::Not(inner) => collect_atoms(inner, !neg, out),
        Formula::And(v) | Formula::Or(v) => {
            for x in v {
                collect_atoms(x, neg, out);
            }
        }
        Formula::Forall(_, inner) | Formula::Exists(_, inner) | Formula::Pref(_, inner) => {
            collect_atoms(inner, neg, out)
        }
        _ => {}
    }
}

/// Predicate names any action can add or delete (dynamic); the complement of the
/// declared predicates is the static/structural set.
pub fn dynamic_predicates(domain: &Domain) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for a in &domain.actions {
        collect_eff_heads(&a.effect, &mut out);
    }
    for da in &domain.durative_actions {
        for (_, e) in &da.effects {
            collect_eff_heads(e, &mut out);
        }
    }
    out
}

fn collect_eff_heads(e: &Effect, out: &mut BTreeSet<String>) {
    match e {
        Effect::Add(p, _) | Effect::Del(p, _) => {
            out.insert(p.clone());
        }
        Effect::And(v) => {
            for x in v {
                collect_eff_heads(x, out);
            }
        }
        Effect::When(_, inner) | Effect::Forall(_, inner) => collect_eff_heads(inner, out),
        Effect::Num(..) => {}
    }
}

/// Flatten a problem's goal into positive ground atoms `(pred, [args])`
/// (lowercased) — for seeding a visual editor.
pub fn goal_facts(problem: &Problem) -> Vec<(String, Vec<String>)> {
    let mut atoms = Vec::new();
    collect_atoms(&problem.goal, false, &mut atoms);
    atoms
        .into_iter()
        .filter(|(_, _, neg)| !neg)
        .map(|(p, a, _)| {
            (
                p.to_lowercase(),
                a.iter().map(|x| x.to_lowercase()).collect(),
            )
        })
        .collect()
}

/// Generate a PDDL problem (objects grouped by type) — used by the editor.
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

/// Generate a PDDL domain from editable parts (name, requirements, types,
/// predicates) plus action blocks preserved verbatim — used by the editor.
/// `types` is `(child, parent)` (empty parent = top-level); `predicates` is
/// `(name, [arg_types])`; `requirements`/`actions_raw` are raw s-expressions.
pub fn domain_to_pddl(
    name: &str,
    requirements: &str,
    types: &[(String, String)],
    predicates: &[(String, Vec<String>)],
    actions_raw: &[String],
) -> String {
    let mut s = format!("(define (domain {})\n", name.to_lowercase());
    let req = if requirements.trim().is_empty() {
        "(:requirements :strips :typing)"
    } else {
        requirements.trim()
    };
    s.push_str(&format!("  {req}\n"));
    if !types.is_empty() {
        s.push_str("  (:types\n");
        for (t, p) in types {
            if p.trim().is_empty() {
                s.push_str(&format!("    {}\n", t.to_lowercase()));
            } else {
                s.push_str(&format!(
                    "    {} - {}\n",
                    t.to_lowercase(),
                    p.to_lowercase()
                ));
            }
        }
        s.push_str("  )\n");
    }
    if !predicates.is_empty() {
        s.push_str("  (:predicates\n");
        for (n, args) in predicates {
            s.push_str(&format!("    ({}", n.to_lowercase()));
            for (i, t) in args.iter().enumerate() {
                s.push_str(&format!(" ?a{i} - {}", t.to_lowercase()));
            }
            s.push_str(")\n");
        }
        s.push_str("  )\n");
    }
    for a in actions_raw {
        s.push_str("  ");
        s.push_str(a.trim());
        s.push('\n');
    }
    s.push_str(")\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_domain, parse_problem};

    const DOM: &str = "
    (define (domain logi) (:requirements :typing)
      (:types location truck package)
      (:predicates (at ?x - truck ?l - location) (road ?a ?b - location)
                   (in ?p - package ?t - truck) (delivered ?p - package))
      (:action drive :parameters (?t - truck ?from ?to - location)
        :precondition (and (at ?t ?from) (road ?from ?to))
        :effect (and (not (at ?t ?from)) (at ?t ?to))))";
    const PRB: &str = "
    (define (problem p) (:domain logi)
      (:objects a b c - location  t1 - truck  p1 - package)
      (:init (at t1 a) (road a b) (road b c) (in p1 t1))
      (:goal (delivered p1)))";

    #[test]
    fn classification() {
        let d = parse_domain(DOM).unwrap();
        let p = parse_problem(PRB).unwrap();
        let g = VizGraph::build(&d, &p);
        let nodes: BTreeSet<_> = g.nodes.iter().map(|n| n.object.clone()).collect();
        assert_eq!(
            nodes,
            ["A", "B", "C"].iter().map(|s| s.to_string()).collect()
        );
        assert_eq!(g.edges.len(), 2);
        let t1 = g.mobiles.iter().find(|m| m.object == "T1").unwrap();
        assert_eq!(t1.at.as_deref(), Some("A"));
        let p1 = g.mobiles.iter().find(|m| m.object == "P1").unwrap();
        assert_eq!(
            p1.at.as_deref(),
            Some("A"),
            "package in truck resolves to its node"
        );
        assert!(g.goal_by_object.contains_key("P1"));
        let dyn_ = dynamic_predicates(&d);
        assert!(dyn_.contains("AT") && !dyn_.contains("ROAD"));
    }

    #[test]
    fn positions_from_facts() {
        let d = parse_domain(DOM).unwrap();
        let p = parse_problem(PRB).unwrap();
        let g = VizGraph::build(&d, &p);
        let pos = g.positions_at(&["(AT T1 B)".to_string(), "(IN P1 T1)".to_string()]);
        assert_eq!(pos.get("T1"), Some(&Some("B".to_string())));
        assert_eq!(
            pos.get("P1"),
            Some(&Some("B".to_string())),
            "package follows truck to B"
        );
    }

    #[test]
    fn pddl_round_trips() {
        let objects = vec![
            ("a".into(), "location".into()),
            ("t1".into(), "truck".into()),
        ];
        let init = vec![("at".into(), vec!["t1".into(), "a".into()])];
        let goal = vec![("at".into(), vec!["t1".into(), "a".into()])];
        let pddl = to_pddl("p", "logi", &objects, &init, &goal);
        let parsed = parse_problem(&pddl).expect("must parse");
        assert_eq!(parsed.objects.len(), 2);
        assert!(matches!(parsed.goal, Formula::And(_)));
    }
}
