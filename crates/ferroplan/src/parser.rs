//! Recursive-descent parser for the PDDL subset Metric-FF accepts.
//! Mirrors `scan-ops_pddl.y` (domain) and `scan-fct_pddl.y` (problem).

use crate::lexer::{lex, Tok};
use crate::types::*;

/// The requirements Metric-FF's `supported()` whitelist accepts (uppercased).
const SUPPORTED: &[&str] = &[
    ":STRIPS",
    ":NEGATION",
    ":EQUALITY",
    ":TYPING",
    ":CONDITIONAL-EFFECTS",
    ":NEGATIVE-PRECONDITIONS",
    ":DISJUNCTIVE-PRECONDITIONS",
    ":EXISTENTIAL-PRECONDITIONS",
    ":UNIVERSAL-PRECONDITIONS",
    ":QUANTIFIED-PRECONDITIONS",
    ":ADL",
    ":FLUENTS",
    ":NUMERIC-FLUENTS",
    ":ACTION-COSTS",
    // PDDL3.0
    ":PREFERENCES",
    ":CONSTRAINTS",
    ":GOAL-UTILITIES",
];

struct P {
    t: Vec<Tok>,
    i: usize,
}

impl P {
    fn new(t: Vec<Tok>) -> Self {
        P { t, i: 0 }
    }
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn next(&mut self) -> Result<Tok, String> {
        let t = self
            .t
            .get(self.i)
            .cloned()
            .ok_or_else(|| "unexpected end of input".to_string())?;
        self.i += 1;
        Ok(t)
    }
    fn expect_lparen(&mut self) -> Result<(), String> {
        match self.next()? {
            Tok::LParen => Ok(()),
            other => Err(format!("expected '(', found {:?}", other)),
        }
    }
    fn expect_rparen(&mut self) -> Result<(), String> {
        match self.next()? {
            Tok::RParen => Ok(()),
            other => Err(format!("expected ')', found {:?}", other)),
        }
    }
    fn at_rparen(&self) -> bool {
        matches!(self.peek(), Some(Tok::RParen))
    }
    fn num(&mut self) -> Result<f64, String> {
        match self.next()? {
            Tok::Num(n) => Ok(n),
            other => Err(format!("expected number, found {:?}", other)),
        }
    }
    /// Consume a Name token, returning its (uppercase) text.
    fn name(&mut self) -> Result<String, String> {
        match self.next()? {
            Tok::Name(s) => Ok(s),
            other => Err(format!("expected name, found {:?}", other)),
        }
    }
    /// Consume `(`, expect a specific keyword name, leave cursor after it.
    fn expect_kw(&mut self, kw: &str) -> Result<(), String> {
        let n = self.name()?;
        if n == kw {
            Ok(())
        } else {
            Err(format!("expected `{}`, found `{}`", kw, n))
        }
    }
    /// Skip a balanced parenthesized form (cursor must be just after its `(`).
    fn skip_balanced(&mut self) -> Result<(), String> {
        let mut depth = 1;
        while depth > 0 {
            match self.next()? {
                Tok::LParen => depth += 1,
                Tok::RParen => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }
}

/// Parse a typed list: `a b - T c - U d` → [(A,T),(B,T),(C,U),(D,OBJECT)].
/// Accepts both Names and Vars as items; vars keep their `?`-stripped name.
fn parse_typed_list(p: &mut P) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    while !p.at_rparen() {
        match p.peek().cloned() {
            Some(Tok::Dash) => {
                p.next()?; // consume '-'
                // a type follows; it may be `(either t1 t2)` — take the first.
                let ty = match p.next()? {
                    Tok::Name(s) => s,
                    Tok::LParen => {
                        // (either T ...) — read names, use the first, skip rest
                        let _either = p.name()?; // EITHER
                        let first = p.name()?;
                        while !p.at_rparen() {
                            p.next()?;
                        }
                        p.expect_rparen()?;
                        first
                    }
                    other => return Err(format!("expected type, found {:?}", other)),
                };
                for nm in pending.drain(..) {
                    out.push((nm, ty.clone()));
                }
            }
            Some(Tok::Name(s)) => {
                p.next()?;
                pending.push(s);
            }
            Some(Tok::Var(s)) => {
                p.next()?;
                pending.push(s);
            }
            other => return Err(format!("unexpected token in typed list: {:?}", other)),
        }
    }
    // leftovers with no explicit type default to OBJECT
    for nm in pending.drain(..) {
        out.push((nm, "OBJECT".to_string()));
    }
    Ok(out)
}

fn term_of(t: Tok) -> Result<Term, String> {
    match t {
        Tok::Var(s) => Ok(Term::Var(s)),
        Tok::Name(s) => Ok(Term::Const(s)),
        other => Err(format!("expected term, found {:?}", other)),
    }
}

fn parse_expr(p: &mut P) -> Result<Expr, String> {
    match p.next()? {
        Tok::Num(n) => Ok(Expr::Num(n)),
        Tok::LParen => {
            // either an operator application or a fluent head
            match p.peek().cloned() {
                Some(Tok::Op(op)) => {
                    p.next()?;
                    // `+` and `*` are n-ary in PDDL (fold left); `/` is binary
                    if op == "+" || op == "*" {
                        let mut acc = parse_expr(p)?;
                        while !p.at_rparen() {
                            let b = parse_expr(p)?;
                            acc = if op == "+" {
                                Expr::Add(Box::new(acc), Box::new(b))
                            } else {
                                Expr::Mul(Box::new(acc), Box::new(b))
                            };
                        }
                        p.expect_rparen()?;
                        Ok(acc)
                    } else if op == "/" {
                        let a = parse_expr(p)?;
                        let b = parse_expr(p)?;
                        p.expect_rparen()?;
                        Ok(Expr::Div(Box::new(a), Box::new(b)))
                    } else {
                        Err(format!("unexpected operator `{}` in expression", op))
                    }
                }
                Some(Tok::Dash) => {
                    p.next()?;
                    let a = parse_expr(p)?;
                    if p.at_rparen() {
                        p.expect_rparen()?;
                        Ok(Expr::Neg(Box::new(a)))
                    } else {
                        let b = parse_expr(p)?;
                        p.expect_rparen()?;
                        Ok(Expr::Sub(Box::new(a), Box::new(b)))
                    }
                }
                Some(Tok::Name(_)) => {
                    let head = p.name()?;
                    let mut args = Vec::new();
                    while !p.at_rparen() {
                        args.push(term_of(p.next()?)?);
                    }
                    p.expect_rparen()?;
                    Ok(Expr::Fluent(head, args))
                }
                other => Err(format!("unexpected token in expression: {:?}", other)),
            }
        }
        other => Err(format!("expected expression, found {:?}", other)),
    }
}

fn comp_of(op: &str) -> Option<CompOp> {
    match op {
        "<" => Some(CompOp::Lt),
        "<=" => Some(CompOp::Le),
        "=" => Some(CompOp::Eq),
        ">=" => Some(CompOp::Ge),
        ">" => Some(CompOp::Gt),
        _ => None,
    }
}

fn parse_formula(p: &mut P) -> Result<Formula, String> {
    p.expect_lparen()?;
    match p.peek().cloned() {
        Some(Tok::Op(op)) => {
            p.next()?;
            // object equality `(= a b)` when the operands are terms, not numbers
            if op == "=" && matches!(p.peek(), Some(Tok::Var(_)) | Some(Tok::Name(_))) {
                let a = term_of(p.next()?)?;
                let b = term_of(p.next()?)?;
                p.expect_rparen()?;
                return Ok(Formula::Eq(a, b));
            }
            let c = comp_of(&op)
                .ok_or_else(|| format!("unexpected operator `{}` in formula", op))?;
            let a = parse_expr(p)?;
            let b = parse_expr(p)?;
            p.expect_rparen()?;
            Ok(Formula::Comp(c, a, b))
        }
        Some(Tok::Name(head)) => {
            p.next()?;
            match head.as_str() {
                "AND" => {
                    let mut v = Vec::new();
                    while !p.at_rparen() {
                        v.push(parse_formula(p)?);
                    }
                    p.expect_rparen()?;
                    Ok(Formula::And(v))
                }
                "OR" => {
                    let mut v = Vec::new();
                    while !p.at_rparen() {
                        v.push(parse_formula(p)?);
                    }
                    p.expect_rparen()?;
                    Ok(Formula::Or(v))
                }
                "NOT" => {
                    let f = parse_formula(p)?;
                    p.expect_rparen()?;
                    Ok(Formula::Not(Box::new(f)))
                }
                "IMPLY" => {
                    let a = parse_formula(p)?;
                    let b = parse_formula(p)?;
                    p.expect_rparen()?;
                    Ok(Formula::Or(vec![Formula::Not(Box::new(a)), b]))
                }
                "FORALL" | "EXISTS" => {
                    // (forall|exists (typed vars) phi)
                    p.expect_lparen()?;
                    let vars = parse_typed_list(p)?;
                    p.expect_rparen()?;
                    let inner = parse_formula(p)?;
                    p.expect_rparen()?;
                    if head == "FORALL" {
                        Ok(Formula::Forall(vars, Box::new(inner)))
                    } else {
                        Ok(Formula::Exists(vars, Box::new(inner)))
                    }
                }
                "PREFERENCE" => {
                    // (preference [name] phi) — a SOFT goal
                    let name = if matches!(p.peek(), Some(Tok::Name(_))) {
                        Some(p.name()?)
                    } else {
                        None
                    };
                    let f = parse_formula(p)?;
                    p.expect_rparen()?;
                    Ok(Formula::Pref(name, Box::new(f)))
                }
                _ => {
                    // an atom: head is a predicate name
                    let mut args = Vec::new();
                    while !p.at_rparen() {
                        args.push(term_of(p.next()?)?);
                    }
                    p.expect_rparen()?;
                    Ok(Formula::Atom(head, args))
                }
            }
        }
        // an empty `()` precondition means TRUE
        Some(Tok::RParen) => {
            p.expect_rparen()?;
            Ok(Formula::True)
        }
        other => Err(format!("unexpected token in formula: {:?}", other)),
    }
}

fn parse_effect(p: &mut P) -> Result<Effect, String> {
    p.expect_lparen()?;
    match p.peek().cloned() {
        Some(Tok::Name(head)) => {
            p.next()?;
            match head.as_str() {
                "AND" => {
                    let mut v = Vec::new();
                    while !p.at_rparen() {
                        v.push(parse_effect(p)?);
                    }
                    p.expect_rparen()?;
                    Ok(Effect::And(v))
                }
                "NOT" => {
                    // (not (pred args))
                    p.expect_lparen()?;
                    let pred = p.name()?;
                    let mut args = Vec::new();
                    while !p.at_rparen() {
                        args.push(term_of(p.next()?)?);
                    }
                    p.expect_rparen()?; // close inner atom
                    p.expect_rparen()?; // close (not ..)
                    Ok(Effect::Del(pred, args))
                }
                "INCREASE" | "DECREASE" | "ASSIGN" | "SCALE-UP" | "SCALE-DOWN" => {
                    let op = match head.as_str() {
                        "INCREASE" => AssignOp::Increase,
                        "DECREASE" => AssignOp::Decrease,
                        "ASSIGN" => AssignOp::Assign,
                        "SCALE-UP" => AssignOp::ScaleUp,
                        _ => AssignOp::ScaleDown,
                    };
                    // target fluent head
                    p.expect_lparen()?;
                    let fname = p.name()?;
                    let mut fargs = Vec::new();
                    while !p.at_rparen() {
                        fargs.push(term_of(p.next()?)?);
                    }
                    p.expect_rparen()?;
                    let val = parse_expr(p)?;
                    p.expect_rparen()?;
                    Ok(Effect::Num(op, fname, fargs, val))
                }
                "WHEN" => {
                    // (when <condition> <effect>)
                    let cond = parse_formula(p)?;
                    let eff = parse_effect(p)?;
                    p.expect_rparen()?;
                    Ok(Effect::When(cond, Box::new(eff)))
                }
                "FORALL" => {
                    // (forall (typed vars) <effect>)
                    p.expect_lparen()?;
                    let vars = parse_typed_list(p)?;
                    p.expect_rparen()?;
                    let eff = parse_effect(p)?;
                    p.expect_rparen()?;
                    Ok(Effect::Forall(vars, Box::new(eff)))
                }
                _ => {
                    // positive atom add-effect
                    let mut args = Vec::new();
                    while !p.at_rparen() {
                        args.push(term_of(p.next()?)?);
                    }
                    p.expect_rparen()?;
                    Ok(Effect::Add(head, args))
                }
            }
        }
        other => Err(format!("unexpected token in effect: {:?}", other)),
    }
}

fn parse_predicates(p: &mut P) -> Result<Vec<(String, Vec<String>)>, String> {
    // cursor is just after `(:predicates`
    let mut out = Vec::new();
    while !p.at_rparen() {
        p.expect_lparen()?;
        let name = p.name()?;
        let params = parse_typed_list(p)?;
        p.expect_rparen()?;
        out.push((name, params.into_iter().map(|(_, t)| t).collect()));
    }
    p.expect_rparen()?;
    Ok(out)
}

fn parse_functions(p: &mut P) -> Result<Vec<(String, Vec<String>)>, String> {
    // cursor is just after `(:functions`
    let mut out = Vec::new();
    while !p.at_rparen() {
        p.expect_lparen()?;
        let name = p.name()?;
        let params = parse_typed_list(p)?;
        p.expect_rparen()?;
        out.push((name, params.into_iter().map(|(_, t)| t).collect()));
        // an optional `- number` return type may follow at the list level;
        // parse_typed_list already consumed it as a trailing pair if present,
        // which we harmlessly drop here. (number-typed args are ignored.)
        if matches!(p.peek(), Some(Tok::Dash)) {
            p.next()?; // '-'
            let _ = p.name()?; // NUMBER
        }
    }
    p.expect_rparen()?;
    Ok(out)
}

fn parse_action(p: &mut P) -> Result<Action, String> {
    // cursor is just after `(:action`
    let name = p.name()?;
    let mut params = Vec::new();
    let mut precond = Formula::True;
    let mut effect = Effect::And(vec![]);
    while !p.at_rparen() {
        let kw = p.name()?;
        match kw.as_str() {
            ":PARAMETERS" => {
                p.expect_lparen()?;
                params = parse_typed_list(p)?;
                p.expect_rparen()?;
            }
            ":PRECONDITION" => {
                precond = parse_formula(p)?;
            }
            ":EFFECT" => {
                effect = parse_effect(p)?;
            }
            other => return Err(format!("unknown action keyword `{}`", other)),
        }
    }
    p.expect_rparen()?;
    Ok(Action {
        name,
        params,
        precond,
        effect,
    })
}

/// Parse one PDDL3 `(:constraints ...)` constraint formula (modal operators).
/// Phase 1 stores these in the AST; trajectory compilation is a later phase.
fn parse_constraint(p: &mut P) -> Result<Constraint, String> {
    p.expect_lparen()?;
    let head = p.name()?;
    let c = match head.as_str() {
        "AND" => {
            let mut v = Vec::new();
            while !p.at_rparen() {
                v.push(parse_constraint(p)?);
            }
            Constraint::And(v)
        }
        "FORALL" => {
            p.expect_lparen()?;
            let vars = parse_typed_list(p)?;
            p.expect_rparen()?;
            Constraint::Forall(vars, Box::new(parse_constraint(p)?))
        }
        "PREFERENCE" => {
            let name = if matches!(p.peek(), Some(Tok::Name(_))) {
                Some(p.name()?)
            } else {
                None
            };
            Constraint::Pref(name, Box::new(parse_constraint(p)?))
        }
        "ALWAYS" => Constraint::Always(parse_formula(p)?),
        "SOMETIME" => Constraint::Sometime(parse_formula(p)?),
        "AT-MOST-ONCE" => Constraint::AtMostOnce(parse_formula(p)?),
        "SOMETIME-AFTER" => {
            let a = parse_formula(p)?;
            Constraint::SometimeAfter(a, parse_formula(p)?)
        }
        "SOMETIME-BEFORE" => {
            let a = parse_formula(p)?;
            Constraint::SometimeBefore(a, parse_formula(p)?)
        }
        "WITHIN" => {
            let n = p.num()?;
            Constraint::Within(n, parse_formula(p)?)
        }
        "ALWAYS-WITHIN" => {
            let n = p.num()?;
            let a = parse_formula(p)?;
            Constraint::AlwaysWithin(n, a, parse_formula(p)?)
        }
        "HOLD-DURING" => {
            let n1 = p.num()?;
            let n2 = p.num()?;
            Constraint::HoldDuring(n1, n2, parse_formula(p)?)
        }
        "HOLD-AFTER" => {
            let n = p.num()?;
            Constraint::HoldAfter(n, parse_formula(p)?)
        }
        "AT" => {
            let kw = p.name()?;
            if kw != "END" {
                return Err(format!("expected 'end' in (at end ...), found `{}`", kw));
            }
            Constraint::AtEnd(parse_formula(p)?)
        }
        other => return Err(format!("unsupported constraint operator `{}`", other)),
    };
    p.expect_rparen()?;
    Ok(c)
}

pub fn parse_domain(src: &str) -> Result<Domain, String> {
    let mut p = P::new(lex(src)?);
    p.expect_lparen()?;
    p.expect_kw("DEFINE")?;
    p.expect_lparen()?;
    p.expect_kw("DOMAIN")?;
    let name = p.name()?;
    p.expect_rparen()?;

    let mut d = Domain {
        name,
        requirements: Vec::new(),
        types: Vec::new(),
        type_parent: Vec::new(),
        constants: Vec::new(),
        predicates: Vec::new(),
        functions: Vec::new(),
        actions: Vec::new(),
        constraints: Vec::new(),
    };

    while !p.at_rparen() {
        p.expect_lparen()?;
        let section = p.name()?;
        match section.as_str() {
            ":REQUIREMENTS" => {
                while !p.at_rparen() {
                    let r = p.name()?;
                    if !SUPPORTED.contains(&r.as_str()) {
                        return Err(format!("requirement {} not supported by this FF version", r));
                    }
                    d.requirements.push(r);
                }
                p.expect_rparen()?;
            }
            ":TYPES" => {
                let tl = parse_typed_list(&mut p)?;
                for (name, parent) in tl {
                    d.types.push(name.clone());
                    d.type_parent.push((name, parent));
                }
                p.expect_rparen()?;
            }
            ":CONSTANTS" => {
                d.constants = parse_typed_list(&mut p)?;
                p.expect_rparen()?;
            }
            ":PREDICATES" => {
                d.predicates = parse_predicates(&mut p)?;
            }
            ":FUNCTIONS" => {
                d.functions = parse_functions(&mut p)?;
            }
            ":ACTION" => {
                d.actions.push(parse_action(&mut p)?);
            }
            ":CONSTRAINTS" => {
                if !p.at_rparen() {
                    d.constraints.push(parse_constraint(&mut p)?);
                }
                p.expect_rparen()?;
            }
            ":DERIVED" => {
                return Err("derived predicates are not supported by this FF port".to_string());
            }
            _ => {
                // unknown section: skip its remaining balanced content
                p.skip_balanced()?;
            }
        }
    }
    p.expect_rparen()?;
    Ok(d)
}

/// Parse one `:init` element into either an atom or a fluent assignment.
fn parse_init_elt(
    p: &mut P,
    atoms: &mut Vec<(String, Vec<String>)>,
    fluents: &mut Vec<((String, Vec<String>), f64)>,
) -> Result<(), String> {
    p.expect_lparen()?;
    match p.peek().cloned() {
        Some(Tok::Op(op)) if op == "=" => {
            p.next()?;
            // (= (fhead args) number)
            p.expect_lparen()?;
            let fname = p.name()?;
            let mut fargs = Vec::new();
            while !p.at_rparen() {
                fargs.push(name_or_const(p.next()?)?);
            }
            p.expect_rparen()?;
            let v = match p.next()? {
                Tok::Num(n) => n,
                other => return Err(format!("expected number in init `=`, found {:?}", other)),
            };
            p.expect_rparen()?;
            fluents.push(((fname, fargs), v));
            Ok(())
        }
        Some(Tok::Name(_)) => {
            let pred = p.name()?;
            let mut args = Vec::new();
            while !p.at_rparen() {
                args.push(name_or_const(p.next()?)?);
            }
            p.expect_rparen()?;
            atoms.push((pred, args));
            Ok(())
        }
        other => Err(format!("unexpected token in :init: {:?}", other)),
    }
}

fn name_or_const(t: Tok) -> Result<String, String> {
    match t {
        Tok::Name(s) => Ok(s),
        other => Err(format!("expected object name, found {:?}", other)),
    }
}

pub fn parse_problem(src: &str) -> Result<Problem, String> {
    let mut p = P::new(lex(src)?);
    p.expect_lparen()?;
    p.expect_kw("DEFINE")?;
    p.expect_lparen()?;
    p.expect_kw("PROBLEM")?;
    let name = p.name()?;
    p.expect_rparen()?;

    let mut prob = Problem {
        name,
        domain_name: String::new(),
        objects: Vec::new(),
        init_atoms: Vec::new(),
        init_fluents: Vec::new(),
        goal: Formula::True,
        constraints: Vec::new(),
        metric: None,
    };

    while !p.at_rparen() {
        p.expect_lparen()?;
        let section = p.name()?;
        match section.as_str() {
            ":DOMAIN" => {
                prob.domain_name = p.name()?;
                p.expect_rparen()?;
            }
            ":REQUIREMENTS" => {
                // problem-file requirements are over-read / ignored
                p.skip_balanced()?;
            }
            ":OBJECTS" => {
                prob.objects = parse_typed_list(&mut p)?;
                p.expect_rparen()?;
            }
            ":INIT" => {
                while !p.at_rparen() {
                    parse_init_elt(&mut p, &mut prob.init_atoms, &mut prob.init_fluents)?;
                }
                p.expect_rparen()?;
            }
            ":GOAL" => {
                prob.goal = parse_formula(&mut p)?;
                p.expect_rparen()?;
            }
            ":CONSTRAINTS" => {
                if !p.at_rparen() {
                    prob.constraints.push(parse_constraint(&mut p)?);
                }
                p.expect_rparen()?;
            }
            ":METRIC" => {
                // (:metric minimize|maximize <expr>)  — expr may use
                // (is-violated NAME) and (total-cost), parsed as fluents.
                let dir = match p.name()?.as_str() {
                    "MINIMIZE" => MetricDir::Minimize,
                    "MAXIMIZE" => MetricDir::Maximize,
                    other => return Err(format!("unknown metric direction `{}`", other)),
                };
                let e = parse_expr(&mut p)?;
                prob.metric = Some((dir, e));
                p.expect_rparen()?;
            }
            _ => {
                p.skip_balanced()?;
            }
        }
    }
    p.expect_rparen()?;
    Ok(prob)
}
