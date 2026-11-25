//! Integration tests exercising each rule against golden-file fixtures.
//!
//! A `.py` case pins its diagnostics alongside its formatted output.
//! `diagnostics.snap` is the `rule@start..end` list `diagnose` collects
//! against the unrewritten source, anchored to the source as written.
//! `lint_findings.snap` renders the lint records against the rewritten
//! output, the buffer the docs site decorates onto its formatted view.

use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use assert_cmd::Command;
use itertools::Itertools;
use prose::{
    config::Config, diagnostics::Diagnostic, findings::lint_records, pipeline::Pipeline,
    source::Source,
};
use prose_harness::{
    FIXTURES, HarnessOptions, build_pipeline, case_name, domain_name, fixture_inputs,
    in_snapshot_dir, python_files, unified_diff,
};
use rstest::rstest;
use ruff_python_ast::PySourceType;
use ruff_source_file::{LineEnding, UniversalNewlines};

/// Every fixture [`prose_is_stable_after_ruff`] sweeps as `ruff format`
/// writes it, keyed by [`copy_name`]. One run formats a scratch copy of
/// the whole set, so the cases share a single `ruff` start.
static POST_RUFF: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    let scratch = tempfile::tempdir().expect("tempdir");
    let names = python_files(Path::new(FIXTURES))
        .filter(|path| {
            path.ends_with("input.py")
                && domain_name(path) != "identity"
                && !fixture_inputs(path).1.skip_ruff_coexistence
        })
        .map(|path| {
            let name = copy_name(domain_name(&path), case_name(&path));
            fs_err::copy(&path, scratch.path().join(&name)).expect("copy fixture");
            name
        })
        .collect_vec();

    let output = Command::new("ruff")
        .args(["format", "--isolated", "--no-cache"])
        .arg(scratch.path())
        .output()
        .unwrap_or_else(|e| {
            panic!("`ruff` runs from the toolset `.mise/config.toml` pins, as under `mise run rust:suite`: {e}")
        });
    assert!(
        output.status.success(),
        "ruff format failed:\n{}\n\
         set `[harness] skip_ruff_coexistence = true` in the sidecar of the fixture each `<domain>.<case>.py` names to opt it out",
        String::from_utf8_lossy(&output.stderr),
    );
    let summary = String::from_utf8_lossy(&output.stdout);
    let counted: usize = summary
        .split(", ")
        .filter_map(|part| part.split_whitespace().next()?.parse::<usize>().ok())
        .sum();
    assert_eq!(
        counted,
        names.len(),
        "ruff format reported {counted} of the swept copies:\n{summary}",
    );
    names
        .into_iter()
        .map(|name| {
            let text =
                fs_err::read_to_string(scratch.path().join(&name)).expect("read formatted copy");
            (name, text)
        })
        .collect()
});

#[rstest]
fn fixtures(
    #[files("../tests/rust/fixtures/**/input.py")]
    #[files("../tests/rust/fixtures/**/input.ipynb")]
    path: PathBuf,
) {
    let path = path.as_path();
    let domain = domain_name(path);
    let case = case_name(path);
    let input_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture filename is UTF-8");

    let (config, harness) = fixture_inputs(path);
    let pipeline = build_pipeline(domain, &config, &harness);
    let source = accepted(path, &config, domain, case);

    // `diagnose` reads the source as written, so it runs before the
    // rewrite consumes it. A notebook pins its diagnostics through
    // the CLI tests.
    let module = !PySourceType::from(path).is_ipynb();
    let diagnostics = module.then(|| pipeline.diagnose(&source));

    let (formatted, records, _) = pipeline
        .run(source)
        .expect("first pass succeeds on fixture input");
    let output = formatted.text();

    in_snapshot_dir(path, || {
        if let Some(diagnostics) = &diagnostics {
            insta::assert_snapshot!("diagnostics", render(diagnostics));
            let lint = lint_records(formatted.source_file(), &records);
            if !lint.is_empty() {
                let json = serde_json::to_string_pretty(&lint).expect("lint records serialize");
                insta::assert_snapshot!("lint_findings", json);
            }
        }
        // `binding_analysis` pins its `input.py` snapshot as the
        // binding table, so its formatted text is not snapshotted.
        if domain != "binding_analysis" {
            let snapshot = if domain == "notebook" {
                cell_delimited(&formatted)
            } else {
                output.to_owned()
            };
            insta::assert_snapshot!(input_name, snapshot);
        }
    });

    if domain == "binding_analysis" {
        return;
    }

    // A module reparses from its own formatted text. A notebook's
    // cell boundaries do not survive a bare reparse of that
    // concatenated text, so the CLI tests pin its round-trip
    // idempotency.
    if domain != "notebook" {
        assert_settles(&pipeline, output, domain, case, "on second pass");
    }

    let fresh_source = accepted(path, &config, domain, case);
    let (fresh_formatted, _, _) = build_pipeline(domain, &config, &harness)
        .run(fresh_source)
        .expect("fresh pipeline run succeeds");
    assert!(
        fresh_formatted.text() == output,
        "fixture `{domain}/{case}` not deterministic across pipeline instances:\n{}",
        unified_diff(output, fresh_formatted.text()),
    );
}

/// Reads the fixture input at `path` the way the CLI reads it under the
/// case's `config`, panicking where the input does not parse or holds
/// syntax its `target-version` refuses.
fn accepted(path: &Path, config: &Config, domain: &str, case: &str) -> Source {
    Pipeline::with_defaults(config)
        .read_path(path)
        .expect("fixture input reads and parses")
        .unwrap_or_else(|| panic!("fixture `{domain}/{case}` holds syntax its target refuses"))
}

/// Runs `pipeline` over its own `output` and panics where a second
/// pass changes it.
fn assert_settles(pipeline: &Pipeline, output: &str, domain: &str, case: &str, under: &str) {
    let reparsed = output
        .parse::<Source>()
        .expect("formatter output reparses as Python");
    let (second, _, _) = pipeline.run(reparsed).expect("second pass succeeds");
    assert!(
        second.text() == output,
        "fixture `{domain}/{case}` not idempotent {under}:\n{}",
        unified_diff(output, second.text()),
    );
}

/// The domain, case, sidecar options, and default pipeline for `path`,
/// `None` for an `identity` fixture the full-pipeline sweeps pass over.
fn sweepable(path: &Path) -> Option<(&str, &str, HarnessOptions, Pipeline)> {
    let domain = domain_name(path);
    (domain != "identity").then(|| {
        let (config, harness) = fixture_inputs(path);
        (
            domain,
            case_name(path),
            harness,
            Pipeline::with_defaults(&config),
        )
    })
}

/// Renders a formatted notebook as its per-cell source joined by a
/// `# --- cell N ---` banner, so a snapshot shows the cell structure the
/// concatenated buffer hides. The banner numbers each cell after the
/// first, which leads unmarked.
fn cell_delimited(source: &Source) -> String {
    let cells = source.cell_texts();
    if cells.len() <= 1 {
        return source.text().to_owned();
    }
    let mut out = String::new();
    for (i, text) in cells.iter().enumerate() {
        if i > 0 {
            let _ = write!(out, "\n\n# --- cell {} ---\n\n", i + 1);
        }
        out.push_str(text.trim_end_matches('\n'));
    }
    out.push('\n');
    out
}

/// Names the scratch file [`POST_RUFF`] writes for the fixture `domain/case`.
fn copy_name(domain: &str, case: &str) -> String {
    format!("{domain}.{case}.py")
}

/// `text` with every line ending rewritten as CRLF.
fn crlf(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n")
}

/// The one-indexed line of the first ending in `text` that is not
/// `expected`, the mixed ending a rule leaves behind when it writes its
/// own newline into a source that breaks lines another way.
fn foreign_ending_line(text: &str, expected: LineEnding) -> Option<usize> {
    text.universal_newlines()
        .position(|line| line.line_ending().is_some_and(|ending| ending != expected))
        .map(|line| line + 1)
}

/// Renders each diagnostic as `rule@start..end`, sorted so the snapshot
/// holds one order across runs.
fn render(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| {
            format!(
                "{rule}@{start}..{end}",
                rule = d.rule,
                start = u32::from(d.range.start()),
                end = u32::from(d.range.end()),
            )
        })
        .sorted()
        .join("\n")
}

/// Every fixture re-read with CRLF endings, proving each rule writes the
/// ending its source carries. A rule writing an ending of its own leaves
/// the output mixed, which the second pass then rewrites again.
#[rstest]
fn crlf_input_holds_its_endings_and_settles(
    #[files("../tests/rust/fixtures/**/input.py")] path: PathBuf,
) {
    let Some((domain, case, _, pipeline)) = sweepable(&path) else {
        return;
    };
    let lf = fs_err::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture: {e}"));
    let source = crlf(&lf);
    let (first, _, _) = pipeline
        .run(source.parse::<Source>().expect("CRLF input parses"))
        .expect("first CRLF pass succeeds");
    if let Some(line) = foreign_ending_line(first.text(), LineEnding::CrLf) {
        panic!("fixture `{domain}/{case}` broke line {line} with an ending other than CRLF");
    }

    assert_settles(&pipeline, first.text(), domain, case, "on CRLF input");
}

#[rstest]
fn pipeline_is_idempotent(#[files("../tests/rust/fixtures/**/input.py")] path: PathBuf) {
    let Some((domain, case, _, pipeline)) = sweepable(&path) else {
        return;
    };
    let source = Source::from_path(&path).expect("fixture input reads and parses as Python");
    let (first, _, _) = pipeline
        .run(source)
        .expect("first full-pipeline pass succeeds");
    assert_settles(&pipeline, first.text(), domain, case, "under full pipeline");
}

/// Asserts the pipeline settles on each swept fixture once the `ruff`
/// release `.mise/config.toml` pins has formatted it.
#[rstest]
fn prose_is_stable_after_ruff(#[files("../tests/rust/fixtures/**/input.py")] path: PathBuf) {
    let Some((domain, case, harness, pipeline)) = sweepable(&path) else {
        return;
    };
    if harness.skip_ruff_coexistence {
        return;
    }
    let post_ruff = &POST_RUFF[&copy_name(domain, case)];
    let (one, _, _) = pipeline
        .run(
            post_ruff
                .parse::<Source>()
                .expect("ruff output reparses as Python"),
        )
        .expect("prose pipeline succeeds after ruff");

    if matches!(domain, "composition" | "thematic") {
        assert!(
            one.text() != post_ruff,
            "prose was a no-op on `{case}` after ruff, where a {domain} fixture must require transformation",
        );
    }
    assert_settles(&pipeline, one.text(), domain, case, "after ruff");
}
