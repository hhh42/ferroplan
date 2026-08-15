//! The satdiff battery — ferroplan-sat's differential gate.
//!
//! Built BEFORE the absorption (fixtures first): every verdict in
//! `tests/satdiff/*.cnf` must reproduce, and every SAT model must verify
//! against its CNF by direct clause evaluation. The solver is never
//! trusted to referee itself. Three medium (~100k-clause) instances are
//! generated deterministically in code; assumption, incremental and
//! conflict-budget behavior are pinned alongside.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use ferroplan_sat::{dimacs, CnfFormula, ExtendFormula, Lit, Solver, SolverError, Var};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/satdiff")
}

struct Fixture {
    name: String,
    expect_sat: bool,
    unique_model: Option<Vec<Lit>>,
    formula: CnfFormula,
}

fn load_fixtures() -> Vec<Fixture> {
    let mut names: Vec<PathBuf> = fs::read_dir(fixture_dir())
        .expect("satdiff fixture dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "cnf"))
        .collect();
    names.sort();
    assert!(
        names.len() >= 15,
        "the battery must carry at least 15 small CNFs, found {}",
        names.len()
    );

    names
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = fs::read_to_string(&path).expect("fixture readable");

            let mut expect_sat = None;
            let mut unique_model = None;
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("c expect:") {
                    expect_sat = Some(match rest.trim() {
                        "SAT" => true,
                        "UNSAT" => false,
                        other => panic!("{name}: bad expect header {other:?}"),
                    });
                }
                if let Some(rest) = line.strip_prefix("c unique-model:") {
                    unique_model = Some(
                        rest.split_whitespace()
                            .map(|tok| {
                                Lit::from_dimacs(
                                    tok.parse().expect("unique-model literals are integers"),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                }
            }

            let formula =
                dimacs::parse_dimacs_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let expect_sat =
                expect_sat.unwrap_or_else(|| panic!("{name}: no `c expect:` header"));
            Fixture {
                name,
                expect_sat,
                unique_model,
                formula,
            }
        })
        .collect()
}

/// The referee: verify a model against a formula by direct clause
/// evaluation. Never trusts the solver.
fn verify_model(formula: &CnfFormula, model: &[Lit]) -> Result<(), String> {
    let mut assignment: HashMap<usize, bool> = HashMap::new();
    for &lit in model {
        if let Some(prev) = assignment.insert(lit.index(), lit.is_positive()) {
            if prev != lit.is_positive() {
                return Err(format!("model assigns both polarities to {:?}", lit.var()));
            }
        }
    }
    for (clause_index, clause) in formula.iter().enumerate() {
        let satisfied = clause
            .iter()
            .any(|lit| assignment.get(&lit.index()) == Some(&lit.is_positive()));
        if !satisfied {
            return Err(format!(
                "clause {clause_index} {clause:?} is not satisfied by the model"
            ));
        }
    }
    Ok(())
}

fn solve_formula(formula: &CnfFormula) -> (bool, Option<Vec<Lit>>) {
    let mut solver = Solver::new();
    solver.add_formula(formula);
    let verdict = solver.solve().expect("no budget set, solve must finish");
    let model = if verdict {
        Some(solver.model().expect("SAT verdict must produce a model"))
    } else {
        assert!(solver.model().is_none(), "UNSAT verdict must not carry a model");
        None
    };
    (verdict, model)
}

#[test]
fn satdiff_small_battery() {
    let fixtures = load_fixtures();
    let mut sat_count = 0;
    let mut unsat_count = 0;

    for fixture in &fixtures {
        let (verdict, model) = solve_formula(&fixture.formula);
        assert_eq!(
            verdict, fixture.expect_sat,
            "{}: verdict differs from the recorded one",
            fixture.name
        );
        if let Some(model) = model {
            sat_count += 1;
            verify_model(&fixture.formula, &model)
                .unwrap_or_else(|e| panic!("{}: {e}", fixture.name));
            if let Some(expected) = &fixture.unique_model {
                let mut got = model.clone();
                let mut want = expected.clone();
                got.sort();
                want.sort();
                assert_eq!(got, want, "{}: model is forced but differs", fixture.name);
            }
        } else {
            unsat_count += 1;
        }
    }

    // The battery must exercise both verdicts substantially.
    assert!(sat_count >= 6, "battery carries too few SAT cases: {sat_count}");
    assert!(unsat_count >= 6, "battery carries too few UNSAT cases: {unsat_count}");
}

#[test]
fn satdiff_empty_formula_is_sat() {
    let mut solver = Solver::new();
    assert_eq!(solver.solve().ok(), Some(true));
    let model = solver.model().expect("SAT must produce a model");
    assert!(model.is_empty(), "no variables were ever mentioned");
}

// --- medium instances (generated deterministically, ~100k clauses) ---

/// xorshift64* — deterministic, dependency-free PRNG for the generators.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut s = self.0;
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        self.0 = s;
        s.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

fn dimacs_lit(number: isize) -> Lit {
    Lit::from_dimacs(number)
}

/// Implication chain x1 -> x2 -> ... -> xn with x1 forced true and xn
/// forced false: UNSAT by pure unit propagation, ~100k clauses.
fn medium_chain_unsat() -> CnfFormula {
    let n: isize = 100_000;
    let mut formula = CnfFormula::new();
    formula.add_clause(&[dimacs_lit(1)]);
    for i in 1..n {
        formula.add_clause(&[dimacs_lit(-i), dimacs_lit(i + 1)]);
    }
    formula.add_clause(&[dimacs_lit(-n)]);
    formula
}

/// Planted random 3-SAT: 40k vars, 100k clauses, seeded; every clause is
/// patched to be satisfied by the planted assignment, so SAT is
/// guaranteed and the model check has teeth.
fn medium_planted_3sat() -> CnfFormula {
    let vars = 40_000usize;
    let clauses = 100_000usize;
    let mut rng = XorShift(0x5EED_0024_5EED_0024);
    let planted: Vec<bool> = (0..vars).map(|_| rng.bool()).collect();

    let mut formula = CnfFormula::new();
    formula.set_var_count(vars);
    let mut lits = [Lit::from_index(0, true); 3];
    for _ in 0..clauses {
        let mut idx = [0usize; 3];
        idx[0] = rng.below(vars);
        loop {
            idx[1] = rng.below(vars);
            if idx[1] != idx[0] {
                break;
            }
        }
        loop {
            idx[2] = rng.below(vars);
            if idx[2] != idx[0] && idx[2] != idx[1] {
                break;
            }
        }
        for (slot, &index) in lits.iter_mut().zip(idx.iter()) {
            *slot = Lit::from_index(index, rng.bool());
        }
        if !lits
            .iter()
            .any(|lit| planted[lit.index()] == lit.is_positive())
        {
            let fix = rng.below(3);
            lits[fix] = Lit::from_index(idx[fix], planted[idx[fix]]);
        }
        formula.add_clause(&lits);
    }
    formula
}

/// Planning-shaped layered chain (the same construction as the
/// `plan-chain-*.cnf` fixtures, sized up): 46 facts, horizon 85,
/// exactly-one action per layer, explanatory frames — ~100k clauses, SAT.
fn medium_layered_chain() -> CnfFormula {
    let nfacts = 46usize;
    let nsteps = 85usize;
    let nact = nfacts - 1;
    let fact = |t: usize, i: usize| Lit::from_index(t * nfacts + i, true);
    let base_a = (nsteps + 1) * nfacts;
    let act = |t: usize, i: usize| Lit::from_index(base_a + t * nact + i, true);

    let mut formula = CnfFormula::new();
    formula.add_clause(&[fact(0, 0)]);
    for i in 1..nfacts {
        formula.add_clause(&[!fact(0, i)]);
    }
    formula.add_clause(&[fact(nsteps, nfacts - 1)]);
    for t in 0..nsteps {
        let alo: Vec<Lit> = (0..nact).map(|i| act(t, i)).collect();
        formula.add_clause(&alo);
        for i in 0..nact {
            for j in i + 1..nact {
                formula.add_clause(&[!act(t, i), !act(t, j)]);
            }
        }
        for i in 0..nact {
            formula.add_clause(&[!act(t, i), fact(t, i)]);
            formula.add_clause(&[!act(t, i), fact(t + 1, i + 1)]);
        }
        for k in 0..nfacts {
            if k == 0 {
                formula.add_clause(&[!fact(t + 1, 0), fact(t, 0)]);
            } else {
                formula.add_clause(&[!fact(t + 1, k), fact(t, k), act(t, k - 1)]);
            }
            formula.add_clause(&[!fact(t, k), fact(t + 1, k)]);
        }
    }
    formula
}

fn run_medium(name: &str, formula: CnfFormula, expect_sat: bool) {
    let start = Instant::now();
    let (verdict, model) = solve_formula(&formula);
    let elapsed = start.elapsed();
    eprintln!(
        "satdiff medium {name}: {} vars, {} clauses, verdict {} in {:.3}s",
        formula.var_count(),
        formula.len(),
        if verdict { "SAT" } else { "UNSAT" },
        elapsed.as_secs_f64()
    );
    assert_eq!(verdict, expect_sat, "{name}: verdict differs");
    if let Some(model) = model {
        verify_model(&formula, &model).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn satdiff_medium_chain_unsat() {
    run_medium("chain-unsat", medium_chain_unsat(), false);
}

#[test]
fn satdiff_medium_planted_3sat() {
    run_medium("planted-3sat", medium_planted_3sat(), true);
}

#[test]
fn satdiff_medium_layered_chain() {
    run_medium("layered-chain", medium_layered_chain(), true);
}

// --- assumptions, incremental use, conflict budget ---

/// Pigeonhole clauses (p pigeons, h holes), each clause guarded by `!enable`.
fn conditional_php(p: usize, h: usize, enable: Lit) -> CnfFormula {
    let var = |i: usize, j: usize| Lit::from_index(i * h + j, true);
    let mut formula = CnfFormula::new();
    for i in 0..p {
        let mut clause: Vec<Lit> = (0..h).map(|j| var(i, j)).collect();
        clause.push(!enable);
        formula.add_clause(&clause);
    }
    for j in 0..h {
        for i1 in 0..p {
            for i2 in i1 + 1..p {
                formula.add_clause(&[!var(i1, j), !var(i2, j), !enable]);
            }
        }
    }
    formula
}

#[test]
fn satdiff_assumptions_failed_core() {
    let enable = Lit::from_index(4 * 3, true);
    let formula = conditional_php(4, 3, enable);

    let mut solver = Solver::new();
    solver.add_formula(&formula);

    // Without assumptions the guard can be false: SAT.
    assert_eq!(solver.solve().ok(), Some(true));

    // Assuming the guard arms the pigeonhole contradiction.
    solver.assume(&[enable]);
    assert_eq!(solver.solve().ok(), Some(false));
    let core = solver
        .failed_core()
        .expect("UNSAT under assumptions must name a failed core");
    assert!(!core.is_empty(), "failed core must not be empty");
    assert!(
        core.iter().all(|lit| *lit == enable),
        "failed core {core:?} must be a subset of the assumptions"
    );

    // Dropping the assumptions restores SAT — the solver stays usable.
    solver.assume(&[]);
    assert_eq!(solver.solve().ok(), Some(true));

    // Hard-asserting the guard makes it plain UNSAT with an empty core.
    solver.add_clause(&[enable]);
    assert_eq!(solver.solve().ok(), Some(false));
    assert_eq!(solver.failed_core(), Some(&[][..]));
}

#[test]
fn satdiff_incremental_clause_adding() {
    // php(5,4): SAT stays SAT while clauses arrive, flips to UNSAT once,
    // then never flips back — the monotone re-solve discipline the
    // horizon ramp relies on.
    let no_guard = Lit::from_index(5 * 4, true);
    let formula = conditional_php(5, 4, no_guard);

    let mut solver = Solver::new();
    solver.add_clause(&[no_guard]);

    let mut last = true;
    let mut flips = 0;
    for clause in formula.iter() {
        solver.add_clause(clause);
        let verdict = solver.solve().expect("no budget set");
        if verdict != last {
            assert!(last && !verdict, "verdict may only flip SAT -> UNSAT");
            flips += 1;
            last = verdict;
        }
    }
    assert_eq!(flips, 1, "the full pigeonhole must end UNSAT");
    assert!(!last);
}

#[test]
fn satdiff_conflict_budget_interrupts() {
    let no_guard = Lit::from_index(8 * 7, true);
    let formula = conditional_php(8, 7, no_guard);

    let mut solver = Solver::new();
    solver.add_clause(&[no_guard]);
    solver.add_formula(&formula);

    // php(8,7) cannot be refuted within 5 conflicts.
    solver.set_conflict_limit(Some(5));
    match solver.solve() {
        Err(SolverError::Interrupted) => {}
        other => panic!("expected budget exhaustion, got {other:?}"),
    }

    // The interrupt is recoverable: lift the budget and finish honestly.
    solver.set_conflict_limit(None);
    assert_eq!(solver.solve().ok(), Some(false));
}

#[test]
fn satdiff_var_api_matches_dimacs_names() {
    // The Var/Lit DIMACS conventions the encoder will lean on.
    assert_eq!(Var::from_dimacs(3).index(), 2);
    assert_eq!(dimacs_lit(-3).var(), Var::from_dimacs(3));
    assert!(dimacs_lit(-3).is_negative());
    let mut formula = CnfFormula::new();
    let v = formula.new_var();
    assert_eq!(v.index(), 0);
}
