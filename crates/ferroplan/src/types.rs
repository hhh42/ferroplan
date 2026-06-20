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

/// A PDDL2.1 `:durative-action`.
#[derive(Clone, Debug)]
pub struct DurativeAction {
    pub name: Sym,
    pub params: Vec<(Sym, Sym)>,
    /// Duration expression from `(= ?duration expr)`. (Duration-inequalities are
    /// not yet supported; only a fixed `=` duration is parsed.)
    pub duration: Expr,
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
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub name: Sym,
    pub domain_name: Sym,
    pub objects: Vec<(Sym, Sym)>,
    pub init_atoms: Vec<(Sym, Vec<Sym>)>,
    pub init_fluents: Vec<((Sym, Vec<Sym>), f64)>,
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
