//! Independent exact-source capability admission verifier.
//!
//! The capability manifest cannot author its own `ADMITTED` state. This binary
//! consumes evidence identifiers produced by completed verification commands,
//! recomputes the manifest fingerprint, and exits successfully only when every
//! shipped capability's evidence set closes.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use ferroplan::{evaluate_readiness, ReadinessState};

#[derive(Debug)]
struct Args {
    source: String,
    evidence_files: Vec<PathBuf>,
    pretty: bool,
}

fn usage() -> &'static str {
    "usage: ferroplan-readiness --source <source-identity> --evidence <file> [--evidence <file> ...] [--compact]"
}

fn parse_args() -> Result<Args, String> {
    let mut source = None;
    let mut evidence_files = Vec::new();
    let mut pretty = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--source requires a value".to_string())?;
                if value.trim().is_empty() {
                    return Err("--source may not be empty".to_string());
                }
                source = Some(value);
            }
            "--evidence" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--evidence requires a file path".to_string())?;
                evidence_files.push(PathBuf::from(value));
            }
            "--compact" => pretty = false,
            "-h" | "--help" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument `{other}`; {}", usage())),
        }
    }
    let source = source.ok_or_else(|| format!("missing --source; {}", usage()))?;
    if evidence_files.is_empty() {
        return Err(format!(
            "at least one --evidence file is required; {}",
            usage()
        ));
    }
    Ok(Args {
        source,
        evidence_files,
        pretty,
    })
}

fn read_evidence(paths: &[PathBuf]) -> Result<BTreeSet<String>, String> {
    let mut evidence = BTreeSet::new();
    for path in paths {
        let bytes = fs::read(path)
            .map_err(|error| format!("reading evidence file {}: {error}", path.display()))?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(format!(
                "evidence file {} exceeds the 4 MiB verifier limit",
                path.display()
            ));
        }
        let text = String::from_utf8(bytes)
            .map_err(|error| format!("evidence file {} is not UTF-8: {error}", path.display()))?;
        for (line_number, line) in text.lines().enumerate() {
            let value = line.trim();
            if value.is_empty() || value.starts_with('#') {
                continue;
            }
            if value.len() > 256
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
            {
                return Err(format!(
                    "invalid evidence identifier at {}:{}",
                    path.display(),
                    line_number + 1
                ));
            }
            evidence.insert(value.to_string());
        }
    }
    Ok(evidence)
}

fn run() -> Result<i32, String> {
    let args = parse_args()?;
    let evidence = read_evidence(&args.evidence_files)?;
    let report = evaluate_readiness(args.source, evidence)
        .map_err(|error| format!("evaluating capability readiness: {error}"))?;
    let output = if args.pretty {
        serde_json::to_string_pretty(&report)
    } else {
        serde_json::to_string(&report)
    }
    .map_err(|error| format!("serializing readiness report: {error}"))?;
    println!("{output}");
    Ok(if report.overall_state == ReadinessState::Admitted {
        0
    } else {
        2
    })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("ferroplan-readiness: {error}");
            std::process::exit(64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_identifiers_are_strictly_bounded() {
        let good = "core.solve.unit";
        assert!(good
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')));
        let bad = "core.solve.unit; rm -rf /";
        assert!(!bad
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')));
    }
}
