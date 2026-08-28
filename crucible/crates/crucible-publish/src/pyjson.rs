//! Emit JSON the way Python's `json` module emits it.
//!
//! Two correct implementations of the same transform still write different
//! bytes, and a board raw is compared byte-for-byte against the oracle's. Three
//! differences matter here, all of them live in this repo's real data:
//!
//! 1. **Separators.** `json.dumps` defaults to `", "` and `": "`. `serde_json`
//!    compact mode writes `,` and `:`. Every row in every raw carries the
//!    spaced form.
//! 2. **`ensure_ascii=True`.** Python escapes every non-ASCII character as
//!    `\uXXXX`, with a surrogate pair above the BMP. `serde_json` emits UTF-8.
//!    This is not hypothetical: engine notes carry an em dash, and
//!    `benchmarks/ipc-opt-2008-11.jsonl` alone holds 259 escaped sequences.
//! 3. **`indent=1`** for the conditions and history files -- one space per
//!    level, which no pretty-printer defaults to.
//!
//! Getting any of these wrong makes the whole differential read as noise and
//! hides the real drift underneath it.

/// Append `s` as a Python-style JSON string literal, escaping non-ASCII.
pub fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if c.is_ascii() => out.push(c),
            c => {
                // ensure_ascii: above the BMP Python emits a surrogate PAIR,
                // exactly as it would appear in a UTF-16 encoding.
                let n = c as u32;
                if n > 0xFFFF {
                    let v = n - 0x1_0000;
                    out.push_str(&format!("\\u{:04x}", 0xD800 + (v >> 10)));
                    out.push_str(&format!("\\u{:04x}", 0xDC00 + (v & 0x3FF)));
                } else {
                    out.push_str(&format!("\\u{n:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// A `serde_json::Value` in Python's compact form (`", "` / `": "`).
///
/// Object key order is the map's own order, which for `serde_json` with the
/// `preserve_order` feature off is sorted -- so only pass objects whose order
/// does not matter, or build the object text yourself. Rows are built field by
/// field by `raw::write_row`, precisely because their order is load-bearing.
pub fn write_value(v: &serde_json::Value, out: &mut String) {
    match v {
        serde_json::Value::Null => out.push_str("null"),
        serde_json::Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        serde_json::Value::Number(n) => out.push_str(&n.to_string()),
        serde_json::Value::String(s) => write_str(s, out),
        serde_json::Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(x, out);
            }
            out.push(']');
        }
        serde_json::Value::Object(o) => {
            out.push('{');
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_str(k, out);
                out.push_str(": ");
                write_value(x, out);
            }
            out.push('}');
        }
    }
}

/// `json.dump(obj, f, indent=1)` -- one space per level.
///
/// Python's indent mode uses `","` (no trailing space) as the item separator
/// because the newline already separates them, but keeps `": "` for keys.
pub fn write_indent1(v: &serde_json::Value, out: &mut String) {
    write_indented(v, 1, out);
}

fn write_indented(v: &serde_json::Value, depth: usize, out: &mut String) {
    let pad = |n: usize| " ".repeat(n);
    match v {
        serde_json::Value::Array(a) if !a.is_empty() => {
            out.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&pad(depth));
                write_indented(x, depth + 1, out);
            }
            out.push('\n');
            out.push_str(&pad(depth - 1));
            out.push(']');
        }
        serde_json::Value::Object(o) if !o.is_empty() => {
            out.push_str("{\n");
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&pad(depth));
                write_str(k, out);
                out.push_str(": ");
                write_indented(x, depth + 1, out);
            }
            out.push('\n');
            out.push_str(&pad(depth - 1));
            out.push('}');
        }
        // Empty containers stay on one line, as Python's indent mode does.
        other => write_value(other, out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Defends hazard 2: real engine notes carry an em dash, and a raw that
    /// emits it as UTF-8 differs from every committed board.
    #[test]
    fn non_ascii_is_escaped_like_python() {
        let mut s = String::new();
        write_str("node cap reached \u{2014} no certificate", &mut s);
        assert_eq!(s, "\"node cap reached \\u2014 no certificate\"");
    }

    #[test]
    fn astral_characters_become_surrogate_pairs() {
        let mut s = String::new();
        write_str("\u{1F600}", &mut s);
        assert_eq!(s, "\"\\ud83d\\ude00\"");
    }

    /// Defends hazard 1.
    #[test]
    fn separators_are_the_spaced_python_defaults() {
        let v: serde_json::Value = serde_json::json!({"a": 1, "b": [1, 2]});
        let mut s = String::new();
        write_value(&v, &mut s);
        assert_eq!(s, r#"{"a": 1, "b": [1, 2]}"#);
    }

    #[test]
    fn control_characters_match_python() {
        let mut s = String::new();
        write_str("a\u{1}b\nc", &mut s);
        assert_eq!(s, "\"a\\u0001b\\nc\"");
    }
}
