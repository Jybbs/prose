//! Coverage checks pairing the docs site at `site/` with the live
//! rule registry, the primitive surface, and the fixture catalog.

use std::fs;
use std::path::{Path, PathBuf};

use prose::pipeline::Pipeline;
use regex_lite::Regex;

const PRIMITIVE_PAGES: &[&str] = &[
    "binding-analysis",
    "pipeline",
    "rule-id",
    "source",
    "suppression-map",
];

#[test]
fn every_registered_rule_has_a_page() {
    let rules = Path::new(env!("CARGO_MANIFEST_DIR")).join("site/rules");
    for id in Pipeline::known_ids() {
        let page = rules.join(format!("{id}.md"));
        assert!(
            page.is_file(),
            "rule `{id}` registered in `KNOWN_IDS` has no page at `site/rules/{id}.md`"
        );
    }
}

#[test]
fn every_spec_primitive_has_a_page() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("site/primitives");
    for slug in PRIMITIVE_PAGES {
        let page = dir.join(format!("{slug}.md"));
        assert!(
            page.is_file(),
            "primitive `{slug}` has no page at `site/primitives/{slug}.md`"
        );
    }
}

#[test]
fn every_fixture_invocation_resolves() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let site = root.join("site");
    let pattern = Regex::new(r#"<Fixture rule="([^"]+)" case="([^"]+)" /?>"#).unwrap();
    let mut refs = Vec::new();
    walk_markdown(&site, &mut refs);
    assert!(!refs.is_empty(), "found no markdown under `site/`");

    let mut missing = Vec::new();
    for path in &refs {
        let body = fs::read_to_string(path).unwrap();
        for caps in pattern.captures_iter(&body) {
            let rule = caps.get(1).unwrap().as_str();
            let case = caps.get(2).unwrap().as_str();
            let input = root
                .join("tests/fixtures")
                .join(rule)
                .join(format!("{case}.input.py"));
            let snap = root
                .join("tests/snapshots")
                .join(rule)
                .join(format!("{case}.input.py.snap"));
            if !input.is_file() || !snap.is_file() {
                missing.push(format!(
                    "{} -> rule=\"{rule}\" case=\"{case}\"",
                    path.strip_prefix(&site).unwrap().display()
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "`<Fixture>` invocations point at missing fixture or snapshot pairs:\n  {}",
        missing.join("\n  ")
    );
}

fn walk_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_markdown(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
}
