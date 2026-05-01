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
        "composition" => Pipeline::with_defaults(config),
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
        let stem = common::case_stem(self.0)
            .strip_suffix(".input")
            .expect("fixture path ends in .input.py");
        let sidecar = self.0.with_file_name(format!("{stem}.config.toml"));
        match fs_err::read_to_string(&sidecar) {
            Ok(contents) => Config::from_pyproject_str(&contents)
                .unwrap_or_else(|e| panic!("parse sidecar {sidecar:?}: {e}")),
            Err(e) if e.kind() == ErrorKind::NotFound => Config::default(),
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
        let (second, changed) = pipeline.run(reparsed).expect("second pass succeeds");
        assert!(
            !changed,
            "fixture `{directory}/{case}` not idempotent: second pass emitted edits",
        );
        assert!(
            second.text() == output,
            "fixture `{directory}/{case}` not byte-stable on second pass:\n{}",
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
fn prose_is_stable_after_ruff() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    insta::glob!("fixtures/composition/*.input.py", |path| {
        let case = FixturePath(path).case();
        let input = fs_err::read_to_string(path)
            .unwrap_or_else(|e| panic!("read composition fixture: {e}"));

        let post_ruff = format_module_source(&input, PyFormatOptions::default())
            .unwrap_or_else(|e| panic!("ruff format failed on `{case}`: {e}"))
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

        assert!(
            one.text() != post_ruff,
            "prose was a no-op on `{case}` after ruff — composition fixture should require transformation",
        );
        assert!(
            two.text() == one.text(),
            "prose not stable on `{case}` after ruff:\n\
             --- post-ruff (input to prose) ---\n{post_ruff}\
             --- diff between first and second prose pass ---\n{}",
            common::unified_diff(one.text(), two.text()),
        );
    });
}
