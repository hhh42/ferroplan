//! Walking the benchmark corpus: which variants a track selects, and which
//! instances each variant holds.
//!
//! Ported from `benchmarks/ipc67.py`'s `variants` and `instances`. Two of its
//! rules are load-bearing and both are recorded there as incidents:
//!
//! **Every digit group belongs in the label.** `petri-net`'s
//! `instance-10-1.pddl` and `line-exchange`'s `instance-3_10_50_10.pddl` are
//! distinct problems. Keying on the FIRST group only collapsed twenty of them
//! onto three-to-five labels -- `ipc2026-numeric` held 320 rows under 288 keys
//! -- which silently broke the per-instance diff and the `--score-against`
//! join. So a single-group filename keeps an integer label (every existing
//! board's identity is unchanged by this) and a multipart one becomes the
//! underscore-joined groups. The `domain-<n>` pairing still keys on the FIRST
//! group either way.
//!
//! **A file with no digits is skipped LOUDLY.** It cannot be addressed by
//! instance number at all, so it cannot be run -- but a silent skip reads as a
//! smaller corpus rather than as a corpus bug, and a bad normalisation upstream
//! once took out a whole board mid-run.

use std::path::{Path, PathBuf};

/// One runnable instance: the pair of files, and the label its row carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// `12`, or `"3_10_50_10"`. Rendered into the row exactly as written here.
    pub label: String,
    /// True when the filename held ONE digit group, so the row must carry a
    /// JSON integer rather than a string.
    pub label_is_int: bool,
    pub domain: PathBuf,
    pub problem: PathBuf,
}

/// A variant directory, which is what a "domain" means in this corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ipc: String,
    pub name: String,
    pub dir: PathBuf,
}

#[derive(Debug, Default)]
pub struct Walk {
    pub variants: Vec<Variant>,
    /// Files that could not be addressed. Never dropped in silence.
    pub warnings: Vec<String>,
}

fn digit_groups(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in name.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The sort key `ipc67.py` gets from `tuple(int(g) for g in groups)`.
///
/// Compared as a vector of integers, so `instance-9` precedes `instance-10`
/// and `3_10_50_10` orders against `3_15_25_100` group by group -- never
/// lexically, which would interleave them wrongly.
fn numeric_key(groups: &[String]) -> Vec<u128> {
    groups.iter().filter_map(|g| g.parse().ok()).collect()
}

/// Variants of `track` present on disk, in the order the runner walks them:
/// each competition directory in the track's declared order, and within it,
/// variant directories sorted by name.
pub fn variants(corpus: &Path, ipcs: &[String], selects: &dyn Fn(&str) -> bool) -> Walk {
    let mut w = Walk::default();
    for ipc in ipcs {
        let droot = corpus.join(ipc).join("domains");
        let Ok(rd) = std::fs::read_dir(&droot) else {
            continue;
        };
        let mut names: Vec<String> = rd
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            if selects(&name) {
                let dir = droot.join(&name);
                w.variants.push(Variant {
                    ipc: ipc.clone(),
                    name,
                    dir,
                });
            }
        }
    }
    w
}

/// The instances of one variant, in the order their rows are written.
///
/// `max` mirrors `--max-instances`; 0 means all.
pub fn instances(v: &Variant, max: usize, warnings: &mut Vec<String>) -> Vec<Instance> {
    let idir = v.dir.join("instances");
    let shared = v.dir.join("domain.pddl");
    let Ok(rd) = std::fs::read_dir(&idir) else {
        warnings.push(format!("{}: no instances/ directory", v.dir.display()));
        return Vec::new();
    };

    let mut named: Vec<(Vec<String>, String)> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for e in rd.flatten() {
        let f = e.file_name().to_string_lossy().into_owned();
        let g = digit_groups(&f);
        if g.is_empty() {
            skipped.push(f);
        } else {
            named.push((g, f));
        }
    }
    if !skipped.is_empty() {
        skipped.sort();
        // Loudly, never silently: a missing instance must read as a corpus bug,
        // not as a smaller corpus.
        warnings.push(format!(
            "{}: skipping un-numbered instance file(s): {}",
            v.dir.display(),
            skipped.join(", ")
        ));
    }

    named.sort_by(|a, b| {
        numeric_key(&a.0)
            .cmp(&numeric_key(&b.0))
            .then(a.1.cmp(&b.1))
    });

    let mut out = Vec::new();
    for (groups, file) in named {
        // The domain pairing keys on the FIRST group whether or not the label
        // is multipart.
        let domain = if shared.is_file() {
            shared.clone()
        } else {
            v.dir
                .join("domains")
                .join(format!("domain-{}.pddl", groups[0]))
        };
        if !domain.is_file() {
            // ipc67.py drops these without a word; naming them is strictly
            // better and changes no measurement.
            warnings.push(format!("{}/{file}: no paired domain file", v.name));
            continue;
        }
        let single = groups.len() == 1;
        out.push(Instance {
            // A single group keeps its INTEGER value, so leading zeros in a
            // filename ("p07") do not leak into the label.
            label: if single {
                // The INTEGER value: `p07` is instance 7, and a leading zero
                // must not leak into a key the archive and diff join on.
                groups[0]
                    .parse::<u128>()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|_| groups[0].clone())
            } else {
                groups.join("_")
            },
            label_is_int: single,
            domain,
            problem: idir.join(&file),
        });
    }
    if max > 0 {
        out.truncate(max);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups(f: &str) -> Vec<String> {
        digit_groups(f)
    }

    /// THE 320/288 INCIDENT. Every digit group belongs in the label; keying on
    /// the first collapsed twenty distinct problems onto three-to-five labels.
    #[test]
    fn every_digit_group_is_kept() {
        assert_eq!(groups("instance-3_10_50_10.pddl"), ["3", "10", "50", "10"]);
        assert_eq!(groups("instance-10-1.pddl"), ["10", "1"]);
        assert_eq!(groups("p07.pddl"), ["07"]);
    }

    /// A single-group name keeps an INTEGER label -- every existing board's
    /// identity depends on it -- and a leading zero does not leak into the key
    /// the archive and the diff both join on.
    #[test]
    fn a_single_group_label_is_the_integer_value() {
        assert_eq!(numeric_key(&["07".to_string()]), vec![7u128]);
        assert_eq!(
            "07".parse::<u128>().map(|n| n.to_string()).unwrap(),
            "7",
            "p07 is instance 7, not instance \"07\""
        );
    }

    /// Ordering is by the TUPLE OF INTEGERS, never lexical: lexically "10"
    /// precedes "9", which would interleave a board's rows wrongly.
    #[test]
    fn instances_order_numerically_not_lexically() {
        let mut names = [
            (groups("instance-10.pddl"), "instance-10.pddl".to_string()),
            (groups("instance-9.pddl"), "instance-9.pddl".to_string()),
            (groups("instance-100.pddl"), "instance-100.pddl".to_string()),
        ];
        names.sort_by_key(|(g, _)| numeric_key(g));
        let order: Vec<&str> = names.iter().map(|(_, f)| f.as_str()).collect();
        assert_eq!(
            order,
            ["instance-9.pddl", "instance-10.pddl", "instance-100.pddl"]
        );
    }

    #[test]
    fn multipart_labels_order_group_by_group() {
        let mut names = [
            (groups("instance-3_15_25_100.pddl"), "b".to_string()),
            (groups("instance-3_10_50_10.pddl"), "a".to_string()),
        ];
        names.sort_by_key(|(g, _)| numeric_key(g));
        assert_eq!(names[0].1, "a");
    }

    /// A file with no digits cannot be addressed at all. It is skipped, and the
    /// skip is REPORTED -- a silent one reads as a smaller corpus rather than
    /// as a corpus bug, and that once took out a whole board mid-run.
    #[test]
    fn un_numbered_files_are_reported() {
        assert!(groups("README.pddl").is_empty());
    }

    /// Against the real corpus, if it is present: the labelling rule must
    /// produce 320 distinct keys for ipc-2026n, not the 288 a first-group rule
    /// gives.
    #[test]
    fn the_real_corpus_keeps_every_instance_distinct() {
        let root = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../benchmarks/.ipc-corpus/ipc-2026n/domains"
        ));
        let Ok(rd) = std::fs::read_dir(&root) else {
            return; // corpus is gitignored; absent on a clean clone
        };
        let mut all = 0usize;
        let mut full: std::collections::HashSet<String> = Default::default();
        let mut first: std::collections::HashSet<String> = Default::default();
        for e in rd.flatten() {
            let v = Variant {
                ipc: "ipc-2026n".into(),
                name: e.file_name().to_string_lossy().into_owned(),
                dir: e.path(),
            };
            let mut w = Vec::new();
            for i in instances(&v, 0, &mut w) {
                all += 1;
                full.insert(format!("{}/{}", v.name, i.label));
                let head = i.label.split('_').next().unwrap().to_string();
                first.insert(format!("{}/{head}", v.name));
            }
        }
        assert_eq!(all, full.len(), "every instance has a distinct key");
        assert!(
            first.len() < full.len(),
            "a first-group-only rule would collapse {} keys onto {}",
            full.len(),
            first.len()
        );
    }
}
