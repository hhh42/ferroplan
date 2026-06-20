//! Hand-written PDDL tokenizer (mirrors `lex-ops_pddl.l` / `lex-fct_pddl.l`).
//!
//! Faithful quirks reproduced:
//! - every NAME / VARIABLE is uppercased (`strupcase`);
//! - `;` line comments and `"`-delimited block comments are skipped;
//! - commas (and any other stray char) are treated as whitespace;
//! - a name may contain internal `-`/`_` (so `pick-up`, `consumed-resources`,
//!   `load_vehicle` are single tokens), while a standalone `-` is `Dash`
//!   (the type separator / arithmetic minus).

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    LParen,
    RParen,
    Dash,
    Var(String),
    Name(String),
    Num(f64),
    Op(String), // = < <= > >= + * /
}

pub fn lex(input: &str) -> Result<Vec<Tok>, String> {
    let b = input.as_bytes();
    let n = b.len();
    let mut i = 0;
    let mut out = Vec::new();

    let is_name_start = |c: u8| c.is_ascii_alphabetic() || c == b':';
    let is_name_cont =
        |c: u8| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b':';

    while i < n {
        let c = b[i];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' | b',' => {
                i += 1;
            }
            b';' => {
                // line comment to EOL
                while i < n && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' => {
                // block comment until matching quote
                i += 1;
                while i < n && b[i] != b'"' {
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
            }
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b'?' => {
                let start = i + 1;
                let mut j = start;
                while j < n && is_name_cont(b[j]) {
                    j += 1;
                }
                let name = input[start..j].to_ascii_uppercase();
                out.push(Tok::Var(name));
                i = j;
            }
            b'-' => {
                out.push(Tok::Dash);
                i += 1;
            }
            b'+' | b'*' | b'/' => {
                out.push(Tok::Op((c as char).to_string()));
                i += 1;
            }
            b'=' => {
                out.push(Tok::Op("=".to_string()));
                i += 1;
            }
            b'<' | b'>' => {
                if i + 1 < n && b[i + 1] == b'=' {
                    out.push(Tok::Op(format!("{}=", c as char)));
                    i += 2;
                } else {
                    out.push(Tok::Op((c as char).to_string()));
                    i += 1;
                }
            }
            b'0'..=b'9' | b'.' => {
                let start = i;
                let mut j = i;
                while j < n && (b[j].is_ascii_digit() || b[j] == b'.') {
                    j += 1;
                }
                let s = &input[start..j];
                let v: f64 = s
                    .parse()
                    .map_err(|_| format!("bad number literal `{}`", s))?;
                out.push(Tok::Num(v));
                i = j;
            }
            _ if is_name_start(c) => {
                let start = i;
                let mut j = i;
                while j < n && is_name_cont(b[j]) {
                    j += 1;
                }
                let name = input[start..j].to_ascii_uppercase();
                out.push(Tok::Name(name));
                i = j;
            }
            _ => {
                // unrecognized byte: treat as whitespace (comma tolerance et al.)
                i += 1;
            }
        }
    }
    Ok(out)
}
