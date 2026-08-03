//! Coverage checks pairing the docs site at `site/` with the live rule
//! registry and the per-case fixture tree, so a drifted `<Fixture>`
//! pointer or a malformed `meta.toml` fails a test rather than a build.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use ignore::{WalkBuilder, types::TypesBuilder};
use itertools::Itertools;
use prose::pipeline::Pipeline;
use regex_lite::Regex;
use serde::Deserialize;

const INPUT_SNAPS: [(&str, &str); 3] = [
    ("input.py", "input.py.snap"),
    ("input.ipynb", "input.ipynb.snap"),
    ("input.toml", "config.snap"),
];

/// The `[docs]` block every fixture case carries. `title` and
/// `description` document the case, `previewable` gates whether it
/// renders on the docs site, `canonical = true` marks the one lead
/// example per rule page, and `sandbox = true` opts the case into the
/// interactive sandbox's seed pool.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Docs {
    #[serde(default)]
    canonical: bool,
    description: Option<String>,
    #[serde(default)]
    previewable: bool,
    #[serde(default)]
    sandbox: bool,
    title: Option<String>,
}

#[derive(Deserialize)]
struct Meta {
    docs: Docs,
}

/// Each `.snap` in `case_dir` paired with the `input_file` its header
/// names, skipping any snapshot carrying no such line.
fn declared_inputs(case_dir: &Path) -> Vec<(String, String)> {
    fs_err::read_dir(case_dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "snap"))
        .sorted()
        .filter_map(|path| {
            let declared = fs_err::read_to_string(&path)
                .ok()?
                .lines()
                .find_map(|line| line.strip_prefix("input_file: ").map(ToOwned::to_owned))?;
            Some((dir_name(&path), declared))
        })
        .collect()
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .expect("fixture directory name is UTF-8")
        .to_owned()
}

/// True when the rule at `module`, a single file or a directory of
/// them, carries `needle` anywhere inside it.
fn module_carries(module: &Path, needle: &str) -> bool {
    if let Ok(source) = fs_err::read_to_string(module.with_extension("rs")) {
        return source.contains(needle);
    }
    WalkBuilder::new(module)
        .build()
        .flatten()
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .any(|entry| {
            fs_err::read_to_string(entry.path()).is_ok_and(|source| source.contains(needle))
        })
}

/// The docs-site page for `slug`, searched across every family
/// directory.
fn rule_page(rules: &Path, slug: &str) -> Option<PathBuf> {
    subdirs(rules)
        .into_iter()
        .map(|family| family.join(format!("{slug}.md")))
        .find(|page| page.is_file())
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    fs_err::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .sorted()
        .collect()
}

#[test]
fn every_binding_reader_is_listed_on_the_primitive_page() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = fs_err::read_to_string(root.join("../site/primitives/binding-analysis.md"))
        .expect("the primitive page reads");
    let listed = page
        .lines()
        .find_map(|line| line.strip_prefix("consumedBy: ["))
        .expect("the page declares consumedBy");

    for slug in Pipeline::known_ids() {
        let module = root.join(format!("src/rules/{}", slug.as_str().replace('-', "_")));
        assert!(
            !module_carries(&module, "binding_analysis()") || listed.contains(slug.as_str()),
            "rule `{slug}` reads the binding table and is absent from `consumedBy`",
        );
    }
}

#[test]
fn every_lint_emitting_rule_is_named_on_the_exit_code_page() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rules = root.join("../site/rules");
    for slug in Pipeline::known_ids() {
        let module = root.join(format!("src/rules/{}", slug.as_str().replace('-', "_")));
        if !module_carries(&module, "fn lint(&self") {
            continue;
        }
        let page = rule_page(&rules, slug.as_str()).expect("every rule has a page");
        let declares = page.parent().and_then(Path::file_name) == Some("lint".as_ref())
            || fs_err::read_to_string(&page)
                .expect("the rule page reads")
                .lines()
                .any(|line| line.starts_with("lints") && line.contains("true"));
        assert!(
            declares,
            "rule `{slug}` emits a lint, so its page needs the lint family or `lints : true`",
        );
    }
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
    let mut cases = BTreeMap::<String, String>::new();

    for domain_dir in subdirs(&fixtures) {
        let domain = dir_name(&domain_dir);
        domains.insert(domain.clone());
        for case_dir in subdirs(&domain_dir) {
            let case = dir_name(&case_dir);
            let id = format!("{domain}/{case}");
            if let Some(first) = cases.insert(case.clone(), domain.clone()) {
                violations.push(format!(
                    "{id}: case name is already defined under \"{first}\", and a case \
                     name must be unique across the whole fixture tree"
                ));
            }
            if INPUT_SNAPS
                .iter()
                .all(|(input, _)| !case_dir.join(input).is_file())
            {
                violations.push(format!(
                    "{id}: missing input.py, input.ipynb, and input.toml"
                ));
            }
            for (input, snap) in INPUT_SNAPS {
                if case_dir.join(input).is_file() && !case_dir.join(snap).is_file() {
                    violations.push(format!("{id}: {input} without its {snap}"));
                }
            }
            for (snap, declared) in declared_inputs(&case_dir) {
                if Path::new(&declared).parent().map(Path::file_name) != Some(case_dir.file_name())
                {
                    violations.push(format!("{id}: {snap} names `{declared}`"));
                }
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
            if docs.sandbox && !case_dir.join("input.py").is_file() {
                violations.push(format!("{id}: sandbox case must carry an input.py seed"));
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

#[test]
fn every_fixture_invocation_resolves() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let site = root.join("../site");
    let pattern = Regex::new(r#"<Fixture\w* rule="([^"]+)" case="([^"]+)" /?>"#).unwrap();
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
        let body = fs_err::read_to_string(path).unwrap();
        for caps in pattern.captures_iter(&body) {
            let (_, [rule, case]) = caps.extract();
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
    let rules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../site/rules");
    let families = subdirs(&rules);
    for id in Pipeline::known_ids() {
        assert!(
            families
                .iter()
                .any(|dir| dir.join(format!("{id}.md")).is_file()),
            "rule `{id}` registered in `KNOWN_IDS` has no page under `site/rules/<family>/{id}.md`"
        );
    }
}
