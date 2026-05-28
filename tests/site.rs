//! Coverage checks pairing the docs site at `site/` with the live rule
//! registry and the per-case fixture tree, so a drifted `<Fixture>`
//! pointer or a malformed `meta.toml` fails a test rather than a build.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use ignore::{types::TypesBuilder, WalkBuilder};
use prose::pipeline::Pipeline;
use regex_lite::Regex;
use serde::Deserialize;

/// The `[docs]` opt-in a fixture directory carries to surface on its
/// rule page. A secondary example sets `title`, whereas the lead sets
/// `canonical = true` and omits it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Docs {
    #[serde(default)]
    canonical: bool,
    title: Option<String>,
}

#[derive(Deserialize)]
struct Meta {
    docs: Docs,
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .expect("fixture directory name is UTF-8")
        .to_owned()
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    out.sort();
    out
}

#[test]
fn every_case_directory_is_well_formed() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut violations = Vec::new();
    let mut canonical = BTreeMap::<String, usize>::new();
    let mut domains_w_meta = BTreeSet::<String>::new();

    for domain_dir in subdirs(&fixtures) {
        let domain = dir_name(&domain_dir);
        for case_dir in subdirs(&domain_dir) {
            let id = format!("{domain}/{}", dir_name(&case_dir));
            let has_py = case_dir.join("input.py").is_file();
            let has_toml = case_dir.join("input.toml").is_file();

            if !has_py && !has_toml {
                violations.push(format!("{id}: missing input.py and input.toml"));
            }
            if has_py && !case_dir.join("input.py.snap").is_file() {
                violations.push(format!("{id}: input.py without its input.py.snap"));
            }
            if has_toml && !case_dir.join("config.snap").is_file() {
                violations.push(format!("{id}: input.toml without its config.snap"));
            }

            let meta_path = case_dir.join("meta.toml");
            if !meta_path.is_file() {
                continue;
            }
            domains_w_meta.insert(domain.clone());
            let raw = fs::read_to_string(&meta_path).expect("meta.toml reads");
            match toml::from_str::<Meta>(&raw) {
                Err(e) => violations.push(format!("{id}: meta.toml is not the [docs] shape: {e}")),
                Ok(meta) if meta.docs.canonical => {
                    *canonical.entry(domain.clone()).or_default() += 1
                }
                Ok(meta) if meta.docs.title.as_deref().unwrap_or("").trim().is_empty() => {
                    violations.push(format!(
                        "{id}: non-canonical meta.toml lacks a non-empty title"
                    ));
                }
                Ok(_) => {}
            }
        }
    }

    for domain in &domains_w_meta {
        let count = canonical.get(domain).copied().unwrap_or(0);
        if count != 1 {
            violations.push(format!(
                "rule \"{domain}\" resolves {count} canonical cases, expected exactly 1"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "fixture-tree violations:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn every_fixture_invocation_resolves() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let site = root.join("site");
    let pattern = Regex::new(r#"<Fixture rule="([^"]+)" case="([^"]+)" /?>"#).unwrap();
    let mut types = TypesBuilder::new();
    types.add_defaults();
    types.select("markdown");
    let types = types.build().unwrap();

    let mut found_any = false;
    let mut missing = Vec::new();
    for entry in WalkBuilder::new(&site).types(types).build().flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        found_any = true;
        let path = entry.path();
        let body = fs::read_to_string(path).unwrap();
        for caps in pattern.captures_iter(&body) {
            let rule = caps.get(1).unwrap().as_str();
            let case = caps.get(2).unwrap().as_str();
            let dir = root.join("tests/fixtures").join(rule).join(case);
            let input = dir.join("input.py");
            let snap = dir.join("input.py.snap");
            if !input.is_file() || !snap.is_file() {
                missing.push(format!(
                    "{} -> rule=\"{rule}\" case=\"{case}\"",
                    path.strip_prefix(&site).unwrap().display()
                ));
            }
        }
    }
    assert!(found_any, "found no markdown under `site/`");
    assert!(
        missing.is_empty(),
        "`<Fixture>` invocations point at missing fixture or snapshot pairs:\n  {}",
        missing.join("\n  ")
    );
}

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
