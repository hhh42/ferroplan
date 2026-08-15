// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat-formula/src/cnf.rs); the tuple-sugar helpers
// (`new_vars`/`new_lits`) and proptest strategies were left behind.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! CNF formulas.
use std::{cmp::max, fmt, ops::Range};

use crate::lit::{Lit, Var};

/// A formula in conjunctive normal form (CNF).
///
/// Equivalent to `Vec<Vec<Lit>>` but more efficient as it uses a single buffer for all literals.
#[derive(Clone, Default, Eq)]
pub struct CnfFormula {
    var_count: usize,
    literals: Vec<Lit>,
    clause_ranges: Vec<Range<usize>>,
}

impl CnfFormula {
    /// Create an empty CNF formula.
    pub fn new() -> CnfFormula {
        CnfFormula::default()
    }

    /// Number of variables in the formula.
    ///
    /// This also counts missing variables if a variable with a higher index is present.
    /// A vector of this length can be indexed with the variable indices present.
    pub fn var_count(&self) -> usize {
        self.var_count
    }

    /// Increase the number of variables in the formula.
    ///
    /// If the parameter is less than the current variable count do nothing.
    pub fn set_var_count(&mut self, count: usize) {
        self.var_count = max(self.var_count, count)
    }

    /// Number of clauses in the formula.
    pub fn len(&self) -> usize {
        self.clause_ranges.len()
    }

    /// Whether the set of clauses is empty.
    pub fn is_empty(&self) -> bool {
        self.clause_ranges.is_empty()
    }

    /// Iterator over all clauses.
    pub fn iter(&self) -> impl Iterator<Item = &[Lit]> {
        let literals = &self.literals;
        self.clause_ranges
            .iter()
            .map(move |range| &literals[range.clone()])
    }
}

/// Convert an iterable of [`Lit`] slices into a CnfFormula
impl<Clauses, Item> From<Clauses> for CnfFormula
where
    Clauses: IntoIterator<Item = Item>,
    Item: std::borrow::Borrow<[Lit]>,
{
    fn from(clauses: Clauses) -> CnfFormula {
        let mut cnf_formula = CnfFormula::new();
        for clause in clauses {
            cnf_formula.add_clause(clause.borrow());
        }
        cnf_formula
    }
}

impl fmt::Debug for CnfFormula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.var_count(), f)?;
        f.debug_list().entries(self.iter()).finish()
    }
}

impl PartialEq for CnfFormula {
    fn eq(&self, other: &CnfFormula) -> bool {
        self.var_count() == other.var_count()
            && self.clause_ranges.len() == other.clause_ranges.len()
            && self
                .clause_ranges
                .iter()
                .zip(other.clause_ranges.iter())
                .all(|(range_a, range_b)| {
                    self.literals[range_a.clone()] == other.literals[range_b.clone()]
                })
    }
}

/// Extend a formula with new variables and clauses.
pub trait ExtendFormula: Sized {
    /// Appends a clause to the formula.
    fn add_clause(&mut self, literals: &[Lit]);

    /// Add a new variable to the formula and return it.
    fn new_var(&mut self) -> Var;

    /// Add a new variable to the formula and return it as positive literal.
    fn new_lit(&mut self) -> Lit {
        self.new_var().positive()
    }
}

impl ExtendFormula for CnfFormula {
    fn add_clause(&mut self, clause: &[Lit]) {
        let begin = self.literals.len();
        self.literals.extend_from_slice(clause);
        let end = self.literals.len();

        for &lit in self.literals[begin..end].iter() {
            self.var_count = max(lit.index() + 1, self.var_count);
        }

        self.clause_ranges.push(begin..end);
    }

    fn new_var(&mut self) -> Var {
        let var = Var::from_index(self.var_count);
        self.var_count += 1;
        var
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clause(lits: &[isize]) -> Vec<Lit> {
        lits.iter().map(|&l| Lit::from_dimacs(l)).collect()
    }

    #[test]
    fn new_vars() {
        let mut formula = CnfFormula::new();
        let x = formula.new_var();
        let y = formula.new_var();
        let z = formula.new_var();

        assert_ne!(x, y);
        assert_ne!(y, z);
        assert_ne!(x, z);
        assert_eq!(formula.var_count(), 3);
    }

    #[test]
    fn simple_roundtrip() {
        let input: Vec<Vec<Lit>> = [
            &[1, 2, 3][..],
            &[-1, -2][..],
            &[7, 2][..],
            &[][..],
            &[4, 5][..],
        ]
        .iter()
        .map(|c| clause(c))
        .collect();

        let formula = CnfFormula::from(input.iter().map(|c| c.as_slice()));

        for (clause, ref_clause) in formula.iter().zip(input.iter()) {
            assert_eq!(clause, ref_clause.as_slice());
        }

        assert_eq!(formula.var_count(), 7);
        assert_eq!(formula.len(), 5);
    }
}
