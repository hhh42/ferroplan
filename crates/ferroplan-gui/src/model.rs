//! Derive an abstract *visual graph* from a parsed PDDL domain + problem.
//!
//! PDDL carries no layout/visual info, so we classify predicates by structure
//! (arity + argument types) with name heuristics as a fallback:
//!   - an **edge** predicate is binary over two objects of the same "location"
//!     type — `(road ?a ?b)`, `(connected ?x ?y)`;
//!   - a **position** predicate is binary placing a mobile object onto a location
//!     — `(at ?truck ?loc)`, `(in ?pkg ?truck)`;
//!   - everything else is a **property** shown in the inspector.
//!
//! Location-typed objects become draggable **nodes**; the rest become **mobiles**
//! sitting on the node they're positioned at.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use egui::Pos2;
use ferroplan::types::{Domain, Effect, Formula, Problem, Term};

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
pub struct Node {
    pub object: String,
    pub ty: String,
    pub pos: Pos2,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub a: String,
    pub b: String,
    /// Relation name (e.g. ROAD); kept for edge labels/hover (M2 groundwork).
    #[allow(dead_code)]
    pub pred: String,
}

#[derive(Clone, Debug)]
pub struct Mobile {
    pub object: String,
    pub ty: String,
    /// Node (or, transitively, the node) this object sits on; None -> tray.
    pub at: Option<String>,
    /// Raw position target (may be another mobile, e.g. a package in a truck).
    pub at_raw: Option<String>,
}

#[derive(Default, Clone)]
pub struct VizModel {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub mobiles: Vec<Mobile>,
    /// object -> human-readable atoms/fluents it appears in (for the inspector).
    pub props_by_object: BTreeMap<String, Vec<String>>,
    /// object -> goal atoms it appears in.
    pub goal_by_object: BTreeMap<String, Vec<String>>,
    /// predicate name -> classification (groundwork for a future override UI).
    #[allow(dead_code)]
    pub pred_kind: BTreeMap<String, PredKind>,
    /// inferred location/"node" types (groundwork for the override UI).
    #[allow(dead_code)]
    pub location_types: BTreeSet<String>,
}

impl VizModel {
    pub fn build(domain: &Domain, problem: &Problem) -> Self {
        // object -> type (problem objects + domain constants)
        let obj_ty: HashMap<&str, &str> = problem
            .objects
            .iter()
            .chain(domain.constants.iter())
            .map(|(o, t)| (o.as_str(), t.as_str()))
            .collect();

        // 1. classify predicates by their declared signature
        let mut pred_kind: BTreeMap<String, PredKind> = BTreeMap::new();
        let mut location_types: BTreeSet<String> = BTreeSet::new();
        // types that are themselves *positioned* (arg1 of a position pred) are
        // mobiles, never nodes — even if some other pred uses them as a place
        // (e.g. a package `in` a truck makes `truck` look location-ish).
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
                    // position: arg0 is the mobile, arg1 is the place
                    mobile_types.insert(args[0].clone());
                    location_types.insert(args[1].clone());
                    PredKind::Position
                }
            } else {
                PredKind::Property
            };
            pred_kind.insert(name.clone(), kind);
        }
        // mobiles win the tie
        for m in &mobile_types {
            location_types.remove(m);
        }

        // subtype-aware membership of `location_types`
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

        // 2. nodes = location-typed objects; lay them out on a circle
        let mut node_objs: Vec<(&str, &str)> = obj_ty
            .iter()
            .filter(|(_, t)| is_location(t))
            .map(|(o, t)| (*o, *t))
            .collect();
        node_objs.sort();
        let n = node_objs.len().max(1) as f32;
        let radius = (30.0 * n).max(150.0);
        let nodes: Vec<Node> = node_objs
            .iter()
            .enumerate()
            .map(|(i, (o, t))| {
                let a = std::f32::consts::TAU * (i as f32) / n;
                Node {
                    object: (*o).to_string(),
                    ty: (*t).to_string(),
                    pos: Pos2::new(radius * a.cos(), radius * a.sin()),
                }
            })
            .collect();
        let node_set: BTreeSet<&str> = nodes.iter().map(|x| x.object.as_str()).collect();

        // 3+4+5. walk init atoms: edges, position targets, per-object properties
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
                        edges.push(Edge {
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

        // 6. mobiles = non-node objects; resolve their node transitively
        let resolve_node = |start: &str| -> Option<String> {
            let mut cur = start.to_string();
            for _ in 0..64 {
                if node_set.contains(cur.as_str()) {
                    return Some(cur);
                }
                match pos_target.get(&cur) {
                    Some(next) => cur = next.clone(),
                    None => return None,
                }
            }
            None
        };
        let mut mobiles: Vec<Mobile> = obj_ty
            .iter()
            .filter(|(o, _)| !node_set.contains(*o))
            .map(|(o, t)| {
                let at_raw = pos_target.get(*o).cloned();
                let at = at_raw.as_deref().and_then(resolve_node);
                Mobile {
                    object: (*o).to_string(),
                    ty: (*t).to_string(),
                    at,
                    at_raw,
                }
            })
            .collect();
        mobiles.sort_by(|a, b| a.object.cmp(&b.object));

        // 7. goal atoms per object
        let mut goal_by_object: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut goal_atoms = Vec::new();
        collect_atoms(&problem.goal, &mut goal_atoms);
        for (pred, args, neg) in goal_atoms {
            let s = if neg {
                format!("(not {})", fmt_atom(&pred, &args))
            } else {
                fmt_atom(&pred, &args)
            };
            for a in &args {
                goal_by_object.entry(a.clone()).or_default().push(s.clone());
            }
        }

        VizModel {
            nodes,
            edges,
            mobiles,
            props_by_object: props,
            goal_by_object,
            pred_kind,
            location_types,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.mobiles.is_empty()
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

/// Walk a goal formula, collecting (predicate, object-args, negated) for each
/// ground atom.
fn collect_atoms(f: &Formula, out: &mut Vec<(String, Vec<String>, bool)>) {
    collect_atoms_inner(f, false, out)
}

fn collect_atoms_inner(f: &Formula, neg: bool, out: &mut Vec<(String, Vec<String>, bool)>) {
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
        Formula::Not(inner) => collect_atoms_inner(inner, !neg, out),
        Formula::And(v) | Formula::Or(v) => {
            for x in v {
                collect_atoms_inner(x, neg, out);
            }
        }
        Formula::Forall(_, inner) | Formula::Exists(_, inner) => {
            collect_atoms_inner(inner, neg, out)
        }
        Formula::Pref(_, inner) => collect_atoms_inner(inner, neg, out),
        _ => {}
    }
}

/// Predicate names any action can add or delete (dynamic / "fluent"). Static
/// predicates (never in an effect) describe the fixed structure.
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

#[cfg(test)]
mod tests {
    use super::*;
    use ferroplan::parser::{parse_domain, parse_problem};

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

    fn names(v: &[Node]) -> BTreeSet<String> {
        v.iter().map(|n| n.object.clone()).collect()
    }

    #[test]
    fn classifies_locations_mobiles_edges() {
        let d = parse_domain(DOM).unwrap();
        let p = parse_problem(PRB).unwrap();
        let m = VizModel::build(&d, &p);

        // locations are nodes; truck/package are NOT nodes (mobiles win the tie)
        assert_eq!(
            names(&m.nodes),
            ["A", "B", "C"].iter().map(|s| s.to_string()).collect()
        );
        // edges: a-b, b-c
        assert_eq!(m.edges.len(), 2);
        // mobiles: t1 on a; p1 (in t1) resolves transitively to a
        let t1 = m.mobiles.iter().find(|x| x.object == "T1").unwrap();
        assert_eq!(t1.at.as_deref(), Some("A"));
        let p1 = m.mobiles.iter().find(|x| x.object == "P1").unwrap();
        assert_eq!(p1.at_raw.as_deref(), Some("T1"));
        assert_eq!(
            p1.at.as_deref(),
            Some("A"),
            "package in truck resolves to truck's node"
        );
    }

    #[test]
    fn goal_and_dynamic_predicates() {
        let d = parse_domain(DOM).unwrap();
        let p = parse_problem(PRB).unwrap();
        let m = VizModel::build(&d, &p);
        // p1 is in the goal (delivered p1)
        assert!(m.goal_by_object.contains_key("P1"));
        // AT is dynamic (drive adds/dels it); ROAD is static
        let dyn_ = dynamic_predicates(&d);
        assert!(dyn_.contains("AT"));
        assert!(!dyn_.contains("ROAD"));
    }
}
