//! AST (parser output) and the numeric intermediate representation shared by
//! grounding and the heuristic. The *grounded* representation is data-oriented
//! and lives in `packed.rs`.

pub type Sym = String;

/// A PDDL parse error with the 1-based source line it occurred on.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
#[error("line {line}: {message}")]
pub struct ParseError {
    pub line: u32,
    pub message: String,
}

impl ParseError {
    pub fn new(line: u32, message: impl Into<String>) -> Self {
        ParseError {
            line,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Var(Sym),
    Const(Sym),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompOp {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(f64),
    Fluent(Sym, Vec<Term>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum Formula {
    And(Vec<Formula>),
    Or(Vec<Formula>),
    Not(Box<Formula>),
    Atom(Sym, Vec<Term>),
    Comp(CompOp, Expr, Expr),
    /// ADL quantified preconditions over typed variables.
    Forall(Vec<(Sym, Sym)>, Box<Formula>),
    Exists(Vec<(Sym, Sym)>, Box<Formula>),
    /// ADL object equality `(= a b)` (distinct from numeric `Comp(Eq, ..)`).
    Eq(Term, Term),
    /// PDDL3 `(preference [name] phi)` — a SOFT goal. Classical planners treat it
    /// as `True` (ignore); the metric/optimizer (sgp) consumes it.
    Pref(Option<Sym>, Box<Formula>),
    True,
    False,
}

/// PDDL3 `:metric` optimization direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricDir {
    Minimize,
    Maximize,
}

/// PDDL3 `(:constraints ...)` trajectory constraint (parsed; trajectory
/// compilation is a later phase — phase 1 handles goal preferences + metric).
#[derive(Clone, Debug)]
pub enum Constraint {
    And(Vec<Constraint>),
    Forall(Vec<(Sym, Sym)>, Box<Constraint>),
    Pref(Option<Sym>, Box<Constraint>),
    Always(Formula),
    Sometime(Formula),
    AtMostOnce(Formula),
    SometimeAfter(Formula, Formula),
    SometimeBefore(Formula, Formula),
    AtEnd(Formula),
    Within(f64, Formula),
    AlwaysWithin(f64, Formula, Formula),
    HoldDuring(f64, f64, Formula),
    HoldAfter(f64, Formula),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Increase,
    Decrease,
    ScaleUp,
    ScaleDown,
}

#[derive(Clone, Debug)]
pub enum Effect {
    Add(Sym, Vec<Term>),
    Del(Sym, Vec<Term>),
    Num(AssignOp, Sym, Vec<Term>, Expr),
    And(Vec<Effect>),
    /// ADL conditional effect `(when condition effect)`.
    When(Formula, Box<Effect>),
    /// ADL universal effect `(forall (vars) effect)`.
    Forall(Vec<(Sym, Sym)>, Box<Effect>),
}

#[derive(Clone, Debug)]
pub struct Action {
    pub name: Sym,
    pub params: Vec<(Sym, Sym)>,
    pub precond: Formula,
    pub effect: Effect,
}

/// When a PDDL2.1 durative-action condition/effect applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeSpec {
    /// `at start`
    Start,
    /// `at end`
    End,
    /// `over all` — an invariant that must hold throughout (conditions only).
    All,
}

/// A durative action's duration constraint. A fixed `(= ?duration e)` sets both
/// bounds to `e`; an inequality leaves the open side `None`. The decision-epoch
/// search commits to the **shortest feasible** duration (the lower bound), and the
/// validator accepts any duration in `[min, max]`.
#[derive(Clone, Debug)]
pub struct Duration {
    /// Lower bound (`>=` / `=`). `None` = unbounded below (only an upper bound given).
    pub min: Option<Expr>,
    /// Upper bound (`<=` / `=`). `None` = unbounded above (only a lower bound given).
    pub max: Option<Expr>,
}

impl Duration {
    /// A fixed duration `(= ?duration e)`.
    pub fn fixed(e: Expr) -> Self {
        Duration {
            min: Some(e.clone()),
            max: Some(e),
        }
    }
    /// The bound the search commits to: the lower bound (shortest feasible) if given,
    /// otherwise the upper bound. `None` only if the duration is entirely unconstrained.
    pub fn chosen(&self) -> Option<&Expr> {
        self.min.as_ref().or(self.max.as_ref())
    }
}

/// A PDDL2.1 `:durative-action`.
#[derive(Clone, Debug)]
pub struct DurativeAction {
    pub name: Sym,
    pub params: Vec<(Sym, Sym)>,
    /// Duration constraint: a fixed `(= ?duration e)` or an inequality range.
    pub duration: Duration,
    pub conditions: Vec<(TimeSpec, Formula)>,
    /// Effects are only `at start` / `at end` (`over all` is not a legal effect).
    pub effects: Vec<(TimeSpec, Effect)>,
}

#[derive(Clone, Debug)]
pub struct Domain {
    pub name: Sym,
    pub requirements: Vec<Sym>,
    pub types: Vec<Sym>,
    pub type_parent: Vec<(Sym, Sym)>,
    pub constants: Vec<(Sym, Sym)>,
    pub predicates: Vec<(Sym, Vec<Sym>)>,
    pub functions: Vec<(Sym, Vec<Sym>)>,
    pub actions: Vec<Action>,
    pub durative_actions: Vec<DurativeAction>,
    pub constraints: Vec<Constraint>,
    /// `:derived` rules (axioms). Compiled away before grounding by
    /// [`crate::derived::compile`]: static rules (body over static facts, e.g.
    /// `reachable` from the map) become init facts; dynamic non-recursive rules
    /// are inlined into preconditions/goals.
    pub derived: Vec<DerivedRule>,
}

/// A PDDL `:derived` rule `(:derived (head ?params) body)`: the head predicate's
/// truth is defined by `body` over its parameters, not by action effects.
#[derive(Clone, Debug)]
pub struct DerivedRule {
    pub head: Sym,
    pub params: Vec<(Sym, Sym)>,
    pub body: Formula,
}

/// A PDDL2.2 timed initial literal: `(at <time> <literal>)` in `:init` — a fact
/// that becomes true (`add`) or false (`!add`) at a fixed absolute `time`,
/// independent of any action. Only meaningful under temporal planning.
#[derive(Clone, Debug)]
pub struct TimedLiteral {
    pub time: f64,
    pub add: bool,
    pub pred: Sym,
    pub args: Vec<Sym>,
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub name: Sym,
    pub domain_name: Sym,
    pub objects: Vec<(Sym, Sym)>,
    pub init_atoms: Vec<(Sym, Vec<Sym>)>,
    pub init_fluents: Vec<((Sym, Vec<Sym>), f64)>,
    /// Timed initial literals (PDDL2.2): exogenous facts scheduled at absolute times.
    pub til: Vec<TimedLiteral>,
    pub goal: Formula,
    pub constraints: Vec<Constraint>,
    pub metric: Option<(MetricDir, Expr)>,
}

// ---------------------------------------------------------------------------
// Numeric IR over grounded fluent ids (used by packed task + heuristic).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum NExpr {
    Num(f64),
    Fluent(u32),
    Add(Box<NExpr>, Box<NExpr>),
    Sub(Box<NExpr>, Box<NExpr>),
    Mul(Box<NExpr>, Box<NExpr>),
    Div(Box<NExpr>, Box<NExpr>),
    Neg(Box<NExpr>),
}

impl NExpr {
    pub fn collect_fluents(&self, out: &mut Vec<u32>) {
        match self {
            NExpr::Num(_) => {}
            NExpr::Fluent(i) => out.push(*i),
            NExpr::Neg(a) => a.collect_fluents(out),
            NExpr::Add(a, b) | NExpr::Sub(a, b) | NExpr::Mul(a, b) | NExpr::Div(a, b) => {
                a.collect_fluents(out);
                b.collect_fluents(out);
            }
        }
    }
    pub fn eval(&self, fv: &[f64], def: &[bool]) -> Option<f64> {
        Some(match self {
            NExpr::Num(n) => *n,
            NExpr::Fluent(i) => {
                let i = *i as usize;
                if !def[i] {
                    return None;
                }
                fv[i]
            }
            NExpr::Neg(a) => -a.eval(fv, def)?,
            NExpr::Add(a, b) => a.eval(fv, def)? + b.eval(fv, def)?,
            NExpr::Sub(a, b) => a.eval(fv, def)? - b.eval(fv, def)?,
            NExpr::Mul(a, b) => a.eval(fv, def)? * b.eval(fv, def)?,
            NExpr::Div(a, b) => a.eval(fv, def)? / b.eval(fv, def)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NumPre {
    pub op: CompOp,
    pub lhs: NExpr,
    pub rhs: NExpr,
}

#[derive(Clone, Debug)]
pub struct NumEff {
    pub op: AssignOp,
    pub target: u32,
    pub value: NExpr,
}

pub fn eval_numpre(np: &NumPre, fv: &[f64], def: &[bool]) -> Option<bool> {
    let l = np.lhs.eval(fv, def)?;
    let r = np.rhs.eval(fv, def)?;
    Some(match np.op {
        CompOp::Lt => l < r,
        CompOp::Le => l <= r,
        CompOp::Eq => (l - r).abs() < 1e-6,
        CompOp::Ge => l >= r,
        CompOp::Gt => l > r,
    })
}
