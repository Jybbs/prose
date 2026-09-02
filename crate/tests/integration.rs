//! Integration tests exercising each rule against golden-file fixtures.
//!
//! A `.py` case pins its diagnostics alongside its formatted output.
//! `diagnostics.snap` is the `rule@start..end` list `diagnose` collects
//! against the unrewritten source, anchored to the source as written.
//! `lint_findings.snap` renders the lint records against the rewritten
//! output, the buffer the docs site decorates onto its formatted view.

mod common;

use std::{fmt::Write, path::Path};

use itertools::Itertools;
use prose::{diagnostics::Diagnostic, pipeline::Pipeline, source::Source};
use ruff_python_formatter::{PyFormatOptions, format_module_source};
use ruff_source_file::{LineEnding, UniversalNewlines};

#[test]
fn fixtures() {
    insta::glob!("fixtures/**/input.{py,ipynb}", |path| {
        let domain = common::domain_name(path);
        let case = common::case_name(path);
        let input_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("fixture filename is UTF-8");

        let (config, harness) = common::fixture_inputs(path);
        let pipeline = common::build_pipeline(domain, &config, &harness);
        let source = Source::from_path(path).expect("fixture input reads and parses");

        // `diagnose` reads the source as written, so it runs before the
        // rewrite consumes it. A notebook pins its diagnostics through
        // the CLI tests.
        let module = path.extension().is_some_and(|ext| ext == "py");
        let diagnostics = module.then(|| pipeline.diagnose(&source));

        let (formatted, records) = pipeline
            .run(source)
            .expect("first pass succeeds on fixture input");
        let output = formatted.text();

        common::in_snapshot_dir(path, || {
            if let Some(diagnostics) = &diagnostics {
                insta::assert_snapshot!("diagnostics", render(diagnostics));
                if let Some(json) =
                    prose::findings::lint_records_json(formatted.source_file(), &records)
                {
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
        // concatenated text, so its round-trip idempotency is pinned by
        // the CLI tests rather than here.
        if domain != "notebook" {
            assert_settles(&pipeline, output, domain, case, "on second pass");
        }

        let fresh_source =
            Source::from_path(path).expect("fixture input re-reads for determinism check");
        let (fresh_formatted, _) = common::build_pipeline(domain, &config, &harness)
            .run(fresh_source)
            .expect("fresh pipeline run succeeds");
        assert!(
            fresh_formatted.text() == output,
            "fixture `{domain}/{case}` not deterministic across pipeline instances:\n{}",
            common::unified_diff(output, fresh_formatted.text()),
        );
    });
}

/// Runs `pipeline` over its own `output` and panics where a second
/// pass changes it.
fn assert_settles(pipeline: &Pipeline, output: &str, domain: &str, case: &str, under: &str) {
    let reparsed = output
        .parse::<Source>()
        .expect("formatter output reparses as Python");
    let (second, _) = pipeline.run(reparsed).expect("second pass succeeds");
    assert!(
        second.text() == output,
        "fixture `{domain}/{case}` not idempotent {under}:\n{}",
        common::unified_diff(output, second.text()),
    );
}

/// The domain, case, sidecar options, and default pipeline for `path`,
/// `None` for an `identity` fixture the full-pipeline sweeps pass over.
fn sweepable(path: &Path) -> Option<(&str, &str, common::HarnessOptions, Pipeline)> {
    let domain = common::domain_name(path);
    (domain != "identity").then(|| {
        let (config, harness) = common::fixture_inputs(path);
        (
            domain,
            common::case_name(path),
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
#[test]
fn crlf_input_holds_its_endings_and_settles() {
    insta::glob!("fixtures/**/input.py", |path| {
        let Some((domain, case, _, pipeline)) = sweepable(path) else {
            return;
        };
        let lf = fs_err::read_to_string(path).unwrap_or_else(|e| panic!("read fixture: {e}"));
        let source = crlf(&lf);
        let (first, _) = pipeline
            .run(source.parse::<Source>().expect("CRLF input parses"))
            .expect("first CRLF pass succeeds");
        if let Some(line) = foreign_ending_line(first.text(), LineEnding::CrLf) {
            panic!("fixture `{domain}/{case}` broke line {line} with an ending other than CRLF");
        }

        assert_settles(&pipeline, first.text(), domain, case, "on CRLF input");
    });
}

#[test]
fn pipeline_is_idempotent() {
    insta::glob!("fixtures/**/input.py", |path| {
        let Some((domain, case, _, pipeline)) = sweepable(path) else {
            return;
        };
        let source = Source::from_path(path).expect("fixture input reads and parses as Python");
        let (first, _) = pipeline
            .run(source)
            .expect("first full-pipeline pass succeeds");
        assert_settles(&pipeline, first.text(), domain, case, "under full pipeline");
    });
}

#[test]
fn prose_is_stable_after_ruff() {
    insta::glob!("fixtures/**/input.py", |path| {
        let Some((domain, case, harness, pipeline)) = sweepable(path) else {
            return;
        };
        if harness.skip_ruff_coexistence {
            return;
        }

        let input = fs_err::read_to_string(path).unwrap_or_else(|e| panic!("read fixture: {e}"));
        let post_ruff = format_module_source(&input, PyFormatOptions::default())
            .unwrap_or_else(|e| {
                panic!(
                    "ruff format failed on `{domain}/{case}`: {e}\n\
                     set `[harness] skip_ruff_coexistence = true` in the sidecar to opt this fixture out",
                )
            })
            .into_code();

        let format = |text: &str| {
            pipeline
                .run(
                    text.parse::<Source>()
                        .expect("prose input reparses as Python"),
                )
                .expect("prose pipeline succeeds after ruff")
                .0
        };
        let one = format(&post_ruff);
        let two = format(one.text());

        if matches!(domain, "composition" | "thematic") {
            assert!(
                one.text() != post_ruff,
                "prose was a no-op on `{case}` after ruff — {domain} fixture should require transformation",
            );
        }
        assert!(
            two.text() == one.text(),
            "prose not stable on `{domain}/{case}` after ruff:\n\
             --- post-ruff (input to prose) ---\n{post_ruff}\
             --- diff between first and second prose pass ---\n{}",
            common::unified_diff(one.text(), two.text()),
        );
    });
}
