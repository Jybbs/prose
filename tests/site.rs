//! Coverage checks pairing the docs site at `site/` with the live
//! rule registry, the primitive surface, and the fixture catalog.

use std::{fs, path::Path};

use ignore::{types::TypesBuilder, WalkBuilder};
use prose::pipeline::Pipeline;
use regex_lite::Regex;

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
