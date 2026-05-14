//! Integration tests exercising each rule against golden-file fixtures.

mod common;

use prose::pipeline::Pipeline;
use prose::source::Source;
use ruff_python_formatter::{format_module_source, PyFormatOptions};

#[test]
fn fixtures() {
    insta::glob!("fixtures/**/*.input.py", |path| {
        let directory = common::directory_name(path);
        let case = common::case_filename(path);
        if directory == "binding_analysis" {
            return;
        }

        let (config, harness) = common::fixture_inputs(path);
        let pipeline = common::build_pipeline(directory, &config, &harness);
        let source = Source::from_path(path).expect("fixture input reads and parses as Python");

        let (formatted, _) = pipeline
            .run(source)
            .expect("first pass succeeds on fixture input");
        let output = formatted.text();

        common::in_snapshot_dir(directory, || {
            insta::assert_snapshot!(case, output);
        });

        let reparsed = output
            .parse::<Source>()
            .expect("formatter output reparses as Python");
        let (second, _) = pipeline.run(reparsed).expect("second pass succeeds");
        assert!(
            second.text() == output,
            "fixture `{directory}/{case}` not idempotent on second pass:\n{}",
            common::unified_diff(output, second.text()),
        );

        let fresh_source =
            Source::from_path(path).expect("fixture input re-reads for determinism check");
        let (fresh_formatted, _) = common::build_pipeline(directory, &config, &harness)
            .run(fresh_source)
            .expect("fresh pipeline run succeeds");
        assert!(
            fresh_formatted.text() == output,
            "fixture `{directory}/{case}` not deterministic across pipeline instances:\n{}",
            common::unified_diff(output, fresh_formatted.text()),
        );
    });
}

#[test]
fn pipeline_is_idempotent() {
    insta::glob!("fixtures/**/*.input.py", |path| {
        let directory = common::directory_name(path);
        let case = common::case_filename(path);
        if directory == "identity" {
            return;
        }

        let (config, _) = common::fixture_inputs(path);
        let pipeline = Pipeline::with_defaults(&config);
        let source = Source::from_path(path).expect("fixture input reads and parses as Python");
        let (first, _) = pipeline
            .run(source)
            .expect("first full-pipeline pass succeeds");
        let reparsed = first
            .text()
            .parse::<Source>()
            .expect("full-pipeline output reparses as Python");
        let (second, _) = pipeline
            .run(reparsed)
            .expect("second full-pipeline pass succeeds");
        assert!(
            second.text() == first.text(),
            "fixture `{directory}/{case}` not idempotent under full pipeline:\n{}",
            common::unified_diff(first.text(), second.text()),
        );
    });
}

#[test]
fn prose_is_stable_after_ruff() {
    insta::glob!("fixtures/**/*.input.py", |path| {
        let directory = common::directory_name(path);
        let case = common::case_filename(path);
        if directory == "identity" {
            return;
        }
        let (config, harness) = common::fixture_inputs(path);
        if harness.skip_ruff_coexistence {
            return;
        }

        let input = fs_err::read_to_string(path).unwrap_or_else(|e| panic!("read fixture: {e}"));
        let post_ruff = format_module_source(&input, PyFormatOptions::default())
            .unwrap_or_else(|e| {
                panic!(
                    "ruff format failed on `{directory}/{case}`: {e}\n\
                     set `[harness] skip_ruff_coexistence = true` in the sidecar to opt this fixture out",
                )
            })
            .into_code();

        let pipeline = Pipeline::with_defaults(&config);
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

        if matches!(directory, "composition" | "thematic") {
            assert!(
                one.text() != post_ruff,
                "prose was a no-op on `{case}` after ruff — {directory} fixture should require transformation",
            );
        }
        assert!(
            two.text() == one.text(),
            "prose not stable on `{directory}/{case}` after ruff:\n\
             --- post-ruff (input to prose) ---\n{post_ruff}\
             --- diff between first and second prose pass ---\n{}",
            common::unified_diff(one.text(), two.text()),
        );
    });
}
