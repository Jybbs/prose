//! Integration tests exercising each rule against golden-file fixtures.

mod common;

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;

use prose::config::Config;
use prose::pipeline::Pipeline;
use prose::source::Source;
use ruff_python_formatter::{format_module_source, PyFormatOptions};

fn build_pipeline(directory: &str, config: &Config) -> Pipeline {
    match directory {
        "composition" | "suppression" => Pipeline::with_defaults(config),
        "identity" => Pipeline::empty(),
        _ => Pipeline::for_rule(directory, config)
            .unwrap_or_else(|| panic!("no rule registered for fixture directory `{directory}`")),
    }
}

struct FixturePath<'a>(&'a Path);

impl<'a> FixturePath<'a> {
    fn case(&self) -> &'a str {
        self.0
            .file_name()
            .and_then(OsStr::to_str)
            .expect("fixture path has a file name")
    }

    fn directory(&self) -> &'a str {
        self.0
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .expect("fixture path has a parent directory name")
    }

    fn config(&self) -> Config {
        self.sidecar_contents()
            .map(|c| {
                Config::from_pyproject_str(&c)
                    .unwrap_or_else(|e| panic!("parse sidecar config: {e}"))
            })
            .unwrap_or_default()
    }

    fn harness_options(&self) -> common::HarnessOptions {
        self.sidecar_contents()
            .as_deref()
            .map(common::parse_harness_options)
            .unwrap_or_default()
    }

    fn sidecar_contents(&self) -> Option<String> {
        let stem = common::case_stem(self.0)
            .strip_suffix(".input")
            .expect("fixture path ends in .input.py");
        let sidecar = self.0.with_file_name(format!("{stem}.config.toml"));
        match fs_err::read_to_string(&sidecar) {
            Ok(c) => Some(c),
            Err(e) if e.kind() == ErrorKind::NotFound => None,
            Err(e) => panic!("read sidecar: {e}"),
        }
    }
}

#[test]
fn fixtures() {
    insta::glob!("fixtures/**/*.input.py", |path| {
        let fixture = FixturePath(path);
        let directory = fixture.directory();
        let case = fixture.case();

        let config = fixture.config();
        let pipeline = build_pipeline(directory, &config);
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
        let (fresh_formatted, _) = build_pipeline(directory, &config)
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
        let fixture = FixturePath(path);
        let directory = fixture.directory();
        let case = fixture.case();
        if directory == "identity" {
            return;
        }

        let pipeline = Pipeline::with_defaults(&fixture.config());
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
        let fixture = FixturePath(path);
        let directory = fixture.directory();
        let case = fixture.case();
        if directory == "identity" || fixture.harness_options().skip_ruff_coexistence {
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

        let pipeline = Pipeline::with_defaults(&fixture.config());
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

        if directory == "composition" {
            assert!(
                one.text() != post_ruff,
                "prose was a no-op on `{case}` after ruff — composition fixture should require transformation",
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
