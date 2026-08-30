//! Tests that the binding table a reparse carries matches the one a
//! fresh read builds, over each corpus input.

use super::*;

/// Every Python module and notebook under the tree
/// `PROSE_SETTLE_CORPUS` names, the fixture tree absent it,
/// ascending by path.
fn corpus_inputs() -> Vec<PathBuf> {
    let root = env::var_os("PROSE_SETTLE_CORPUS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        PathBuf::from,
    );
    walker::walk(&[root])
        .filter_map(|found| match found.expect("the corpus walks") {
            Found::Formattable(path, _) => Some(path),
            Found::PassedLink(_) => None,
        })
        .sorted()
        .collect()
}

#[test]
fn carried_binding_tables_match_the_ones_a_fresh_read_builds() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    let mut carried = false;
    for path in corpus_inputs() {
        let Ok(mut source) = Source::from_path(&path) else {
            continue;
        };
        let gate = compile_gate(&source, pipeline.target_version);
        for rule in &pipeline.rules {
            source.binding_analysis();
            let Some((_, new_text, map)) = woven_groups(&**rule, &source) else {
                continue;
            };
            let Ok(next) = reparse_or_reject(source, new_text, &**rule, &map, gate) else {
                break;
            };
            source = next;
            carried |= source.assert_carried_bindings_are_fresh(&format!(
                "{} past `{}`",
                path.display(),
                rule.id(),
            ));
        }
    }
    assert!(
        carried,
        "the binding table was never carried across the corpus"
    );
}
