//! Walks the per-case fixture tree, failing when a case lacks its input
//! or snapshot, when its `meta.toml` `[docs]` block is malformed, or
//! when a rule resolves other than one canonical case.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use prose::pipeline::Pipeline;
use serde::Deserialize;

/// The `[docs]` block every fixture case carries. `title` and
/// `description` document the case, `previewable` gates whether it
/// renders on the docs site, and `canonical = true` marks the one lead
/// example per rule page.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Docs {
    #[serde(default)]
    canonical: bool,
    description: Option<String>,
    #[serde(default)]
    previewable: bool,
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
    let mut out: Vec<PathBuf> = fs_err::read_dir(dir)
        .unwrap()
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
    let rule_slugs: BTreeSet<String> = Pipeline::known_ids()
        .iter()
        .map(ToString::to_string)
        .collect();
    let mut violations = Vec::new();
    let mut canonical = BTreeMap::<String, usize>::new();
    let mut domains = BTreeSet::<String>::new();

    for domain_dir in subdirs(&fixtures) {
        let domain = dir_name(&domain_dir);
        domains.insert(domain.clone());
        for case_dir in subdirs(&domain_dir) {
            let id = format!("{domain}/{}", dir_name(&case_dir));
            let has_py = case_dir.join("input.py").is_file();
            let has_ipynb = case_dir.join("input.ipynb").is_file();
            let has_toml = case_dir.join("input.toml").is_file();

            if !has_py && !has_ipynb && !has_toml {
                violations.push(format!(
                    "{id}: missing input.py, input.ipynb, and input.toml"
                ));
            }
            if has_py && !case_dir.join("input.py.snap").is_file() {
                violations.push(format!("{id}: input.py without its input.py.snap"));
            }
            if has_ipynb && !case_dir.join("input.ipynb.snap").is_file() {
                violations.push(format!("{id}: input.ipynb without its input.ipynb.snap"));
            }
            if has_toml && !case_dir.join("config.snap").is_file() {
                violations.push(format!("{id}: input.toml without its config.snap"));
            }

            let meta_path = case_dir.join("meta.toml");
            if !meta_path.is_file() {
                violations.push(format!("{id}: missing meta.toml"));
                continue;
            }
            let raw = fs_err::read_to_string(&meta_path).expect("meta.toml reads");
            let docs = match toml::from_str::<Meta>(&raw) {
                Ok(meta) => meta.docs,
                Err(e) => {
                    violations.push(format!("{id}: meta.toml is not the [docs] shape: {e}"));
                    continue;
                }
            };
            if docs.title.as_deref().is_none_or(|t| t.trim().is_empty()) {
                violations.push(format!("{id}: meta.toml lacks a non-empty title"));
            }
            if docs
                .description
                .as_deref()
                .is_none_or(|d| d.trim().is_empty())
            {
                violations.push(format!("{id}: meta.toml lacks a non-empty description"));
            }
            if docs.canonical {
                *canonical.entry(domain.clone()).or_default() += 1;
                if !docs.previewable {
                    violations.push(format!("{id}: canonical case must be previewable"));
                }
            }
        }
    }

    for domain in &domains {
        let count = canonical.get(domain).copied().unwrap_or(0);
        let is_rule_page = rule_slugs.contains(&domain.replace('_', "-"));
        if is_rule_page && count != 1 {
            violations.push(format!(
                "rule \"{domain}\" resolves {count} canonical cases, expected exactly 1"
            ));
        }
        if !is_rule_page && count != 0 {
            violations.push(format!(
                "non-rule domain \"{domain}\" carries {count} canonical cases, expected 0"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "fixture-tree violations:\n  {}",
        violations.join("\n  ")
    );
}
