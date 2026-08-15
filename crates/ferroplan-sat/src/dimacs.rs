//! DIMACS CNF reader and writer.
//!
//! Ferroplan-original (the `varisat-dimacs` crate was not absorbed; this
//! replaces it with a dependency-free reader sized to what the crate
//! actually needs): comment lines, an optional `p cnf` header, clauses as
//! zero-terminated integer runs that may span or share lines.

use std::{fmt, io};

use crate::{CnfFormula, ExtendFormula, Lit, Var};

/// Error while parsing a DIMACS CNF input.
#[derive(Debug)]
pub enum DimacsError {
    /// Underlying IO error.
    Io(io::Error),
    /// Malformed input, with the 1-based line number.
    Parse { line: usize, msg: String },
}

impl fmt::Display for DimacsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DimacsError::Io(err) => write!(f, "io error while reading DIMACS: {err}"),
            DimacsError::Parse { line, msg } => write!(f, "DIMACS parse error, line {line}: {msg}"),
        }
    }
}

impl std::error::Error for DimacsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DimacsError::Io(err) => Some(err),
            DimacsError::Parse { .. } => None,
        }
    }
}

impl From<io::Error> for DimacsError {
    fn from(err: io::Error) -> DimacsError {
        DimacsError::Io(err)
    }
}

/// Parse a DIMACS CNF formula from a reader.
pub fn parse_dimacs(input: impl io::BufRead) -> Result<CnfFormula, DimacsError> {
    let mut formula = CnfFormula::new();
    let mut clause: Vec<Lit> = vec![];
    let mut header_vars: Option<usize> = None;
    let mut header_clauses: Option<usize> = None;

    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let line_no = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('c') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('p') {
            if header_vars.is_some() {
                return Err(DimacsError::Parse {
                    line: line_no,
                    msg: "duplicate header".to_owned(),
                });
            }
            let mut fields = rest.split_whitespace();
            let format = fields.next();
            let vars = fields.next().and_then(|v| v.parse::<usize>().ok());
            let clauses = fields.next().and_then(|v| v.parse::<usize>().ok());
            match (format, vars, clauses, fields.next()) {
                (Some("cnf"), Some(vars), Some(clauses), None) if vars <= Var::max_count() => {
                    header_vars = Some(vars);
                    header_clauses = Some(clauses);
                }
                _ => {
                    return Err(DimacsError::Parse {
                        line: line_no,
                        msg: format!("malformed header {trimmed:?}"),
                    });
                }
            }
            continue;
        }
        for token in trimmed.split_whitespace() {
            let number: isize = token.parse().map_err(|_| DimacsError::Parse {
                line: line_no,
                msg: format!("expected integer literal, found {token:?}"),
            })?;
            if number == 0 {
                formula.add_clause(&clause);
                clause.clear();
            } else {
                let index = number.unsigned_abs() - 1;
                if index >= Var::max_count() {
                    return Err(DimacsError::Parse {
                        line: line_no,
                        msg: format!("literal {number} out of range"),
                    });
                }
                clause.push(Lit::from_dimacs(number));
            }
        }
    }

    if !clause.is_empty() {
        return Err(DimacsError::Parse {
            line: 0,
            msg: "unterminated clause at end of input".to_owned(),
        });
    }
    if let Some(vars) = header_vars {
        formula.set_var_count(vars);
        if formula.var_count() > vars {
            return Err(DimacsError::Parse {
                line: 0,
                msg: format!(
                    "formula uses {} variables but the header declares {vars}",
                    formula.var_count()
                ),
            });
        }
    }
    if let Some(clauses) = header_clauses {
        if formula.len() != clauses {
            return Err(DimacsError::Parse {
                line: 0,
                msg: format!(
                    "formula has {} clauses but the header declares {clauses}",
                    formula.len()
                ),
            });
        }
    }

    Ok(formula)
}

/// Parse a DIMACS CNF formula from a string.
pub fn parse_dimacs_str(input: &str) -> Result<CnfFormula, DimacsError> {
    parse_dimacs(input.as_bytes())
}

/// Write a formula in DIMACS CNF format.
pub fn write_dimacs(target: &mut impl io::Write, formula: &CnfFormula) -> io::Result<()> {
    writeln!(target, "p cnf {} {}", formula.var_count(), formula.len())?;
    for clause in formula.iter() {
        for &lit in clause {
            write!(target, "{} ", lit.to_dimacs())?;
        }
        writeln!(target, "0")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let formula = parse_dimacs_str("c comment\np cnf 3 2\n1 -2 0\n2 3 0\n").unwrap();
        assert_eq!(formula.var_count(), 3);
        assert_eq!(formula.len(), 2);
        let clauses: Vec<Vec<Lit>> = formula.iter().map(|c| c.to_vec()).collect();
        assert_eq!(clauses[0], vec![Lit::from_dimacs(1), Lit::from_dimacs(-2)]);
        assert_eq!(clauses[1], vec![Lit::from_dimacs(2), Lit::from_dimacs(3)]);
    }

    #[test]
    fn parse_multiline_and_empty_clause() {
        let formula = parse_dimacs_str("p cnf 2 3\n1\n2 0 -1\n0\n0\n").unwrap();
        assert_eq!(formula.len(), 3);
        let clauses: Vec<Vec<Lit>> = formula.iter().map(|c| c.to_vec()).collect();
        assert_eq!(clauses[0].len(), 2);
        assert_eq!(clauses[1].len(), 1);
        assert!(clauses[2].is_empty());
    }

    #[test]
    fn parse_errors() {
        assert!(parse_dimacs_str("p cnf 1 1\n1\n").is_err());
        assert!(parse_dimacs_str("p cnf 1 2\n1 0\n").is_err());
        assert!(parse_dimacs_str("p cnf 1 1\nx 0\n").is_err());
        assert!(parse_dimacs_str("p cnf 1 1\n2 0\n").is_err());
        assert!(parse_dimacs_str("p cnf 1 1\np cnf 1 1\n1 0\n").is_err());
    }

    #[test]
    fn roundtrip() {
        let text = "p cnf 4 3\n1 -2 0\n-3 4 0\n2 0\n";
        let formula = parse_dimacs_str(text).unwrap();
        let mut out = vec![];
        write_dimacs(&mut out, &formula).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), text);
    }
}
