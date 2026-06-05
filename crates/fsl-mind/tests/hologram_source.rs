//! SPATIAL ROLE: hologram source guardrails — a banned-pattern scanner over the whole
//! workspace enforcing the FSL Coherence Contract at the source level: FLOW is the sole
//! connective primitive, every file declares its SPATIAL ROLE, and there is no regex.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/crates/fsl-mind
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn collect_rs(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            if p.file_name().map(|n| n == "target").unwrap_or(false) {
                continue;
            }
            collect_rs(&p, out);
        } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push((p.clone(), fs::read_to_string(&p).unwrap()));
        }
    }
}

/// All product source files: every crate's `src/` plus the binary's `src/`. Test files
/// are intentionally excluded — they DEFINE the banned patterns, so scanning them would
/// be self-referential.
fn all_sources() -> Vec<(PathBuf, String)> {
    let root = workspace_root();
    let mut out = vec![];
    if let Ok(rd) = fs::read_dir(root.join("crates")) {
        for e in rd.flatten() {
            collect_rs(&e.path().join("src"), &mut out);
        }
    }
    collect_rs(&root.join("src"), &mut out);
    out
}

#[test]
fn flow_is_the_sole_connective_primitive() {
    // No alternative connective primitive may exist anywhere — only `Flow`.
    let banned = ["pub fn connect(", "pub fn join(", "pub fn attach_to("];
    for (path, src) in all_sources() {
        // the designated scanner DEFINES these literals; skip it (it is itself the guardrail).
        if path.file_name().map(|n| n == "hologram_compliance.rs").unwrap_or(false) {
            continue;
        }
        for b in banned {
            assert!(!src.contains(b), "banned connective `{b}` found in {}", path.display());
        }
    }
}

#[test]
fn every_source_declares_a_spatial_role() {
    // Contract: "Every future file must include a one-line SPATIAL ROLE: comment."
    for (path, src) in all_sources() {
        assert!(
            src.contains("SPATIAL ROLE:"),
            "missing `SPATIAL ROLE:` comment in {}",
            path.display()
        );
    }
}

#[test]
fn no_regex_anywhere() {
    // Contract: "No regex anywhere." Routing is structural (tokenized), never pattern-matched.
    for (path, src) in all_sources() {
        assert!(!src.contains("use regex"), "regex import found in {}", path.display());
    }
    // also reject a regex crate dependency.
    let root = workspace_root();
    let mut tomls = vec![root.join("Cargo.toml")];
    if let Ok(rd) = fs::read_dir(root.join("crates")) {
        for e in rd.flatten() {
            tomls.push(e.path().join("Cargo.toml"));
        }
    }
    for t in tomls {
        if let Ok(s) = fs::read_to_string(&t) {
            assert!(!s.contains("regex"), "regex dependency declared in {}", t.display());
        }
    }
}
