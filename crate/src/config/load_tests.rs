//! Filesystem-loading-surface tests for `Config::load`.

use assert_matches::assert_matches;
use indoc::indoc;
use tempfile::TempDir;

use super::load::ConfigForm;
use super::*;
use crate::testing::{write_dotconfig_prose_toml, write_prose_toml, write_pyproject};

#[test]
fn load_absent_file_returns_defaults() {
    let tmp = TempDir::new().expect("tempdir");
    let config = Config::load(tmp.path()).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(88));
    assert!(config.rules.align_equals.enabled);
}

#[test]
fn load_absent_prose_section_returns_defaults() {
    let tmp = TempDir::new().expect("tempdir");
    write_pyproject(tmp.path(), "[project]\nname = \"x\"\n");

    let config = Config::load(tmp.path()).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(88));
    assert!(config.rules.align_equals.enabled);
}

#[test]
fn load_emits_precedence_notice_when_both_present() {
    let tmp = TempDir::new().expect("tempdir");
    write_prose_toml(tmp.path(), "code-line-length = 120\n");
    write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");

    let mut precedence_notices = 0;
    let config = Config::load_with_notices(tmp.path(), |notice| {
        if matches!(notice, ConfigNotice::Precedence { .. }) {
            precedence_notices += 1;
        }
    })
    .expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    assert_eq!(precedence_notices, 1);
}

#[test]
fn load_empty_prose_toml_returns_defaults_and_stops_walk() {
    let tmp = TempDir::new().expect("tempdir");
    let nested = tmp.path().join("child");
    std::fs::create_dir_all(&nested).expect("nested dirs create");
    write_prose_toml(&nested, "");
    write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");

    let config = Config::load(&nested).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(88));
}

#[test]
fn load_from_a_file_path_walks_from_its_directory() {
    let tmp = TempDir::new().expect("tempdir");
    write_prose_toml(tmp.path(), "code-line-length = 120\n");
    let file = tmp.path().join("mod.py");
    std::fs::write(&file, "x = 1\n").expect("writes");

    let config = Config::load(&file).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn load_malformed_dotconfig_prose_toml_returns_toml_error() {
    let tmp = TempDir::new().expect("tempdir");
    write_dotconfig_prose_toml(tmp.path(), "[this is not valid TOML");

    assert_matches!(Config::load(tmp.path()), Err(ConfigError::Toml(_)));
}

#[test]
fn load_malformed_toml_returns_toml_error() {
    let tmp = TempDir::new().expect("tempdir");
    write_pyproject(tmp.path(), "[this is not valid TOML");

    let result = Config::load(tmp.path());

    assert_matches!(result, Err(ConfigError::Toml(_)));
}

#[test]
fn load_partial_override_preserves_other_rule_defaults() {
    let tmp = TempDir::new().expect("tempdir");
    write_pyproject(
        tmp.path(),
        "[tool.prose.rules.align-equals]\nenabled = false\n",
    );

    let config = Config::load(tmp.path()).expect("loads");

    assert!(!config.rules.align_equals.enabled);
    assert!(config.rules.align_colons.enabled);
    assert!(config.rules.strip_trailing_commas.enabled);
}

#[test]
fn load_per_rule_max_shift_overrides_are_independent() {
    let tmp = TempDir::new().expect("tempdir");
    write_pyproject(
        tmp.path(),
        indoc! {r"
            [tool.prose.rules.align-colons]
            max-shift = false

            [tool.prose.rules.align-equals]
            max-shift = 0
        "},
    );

    let config = Config::load(tmp.path()).expect("loads");

    assert_eq!(config.rules.align_colons.max_shift, MaxShift::Unlimited);
    assert_eq!(config.rules.align_equals.max_shift, MaxShift::NoShift);
}

#[test]
fn load_picks_nearest_ancestor_when_multiple_configs_exist() {
    let tmp = TempDir::new().expect("tempdir");
    let nested = tmp.path().join("a/b");
    std::fs::create_dir_all(&nested).expect("nested dirs create");

    write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");
    write_pyproject(&nested, "[tool.prose]\ncode-line-length = 120\n");

    let config = Config::load(&nested).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn load_picks_nearest_prose_toml_over_ancestor_pyproject() {
    let tmp = TempDir::new().expect("tempdir");
    let nested = tmp.path().join("child");
    std::fs::create_dir_all(&nested).expect("nested dirs create");
    write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");
    write_prose_toml(&nested, "code-line-length = 120\n");

    let config = Config::load(&nested).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn load_prefers_dotconfig_over_sibling_pyproject() {
    let tmp = TempDir::new().expect("tempdir");
    write_dotconfig_prose_toml(tmp.path(), "code-line-length = 120\n");
    write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");

    let mut precedence = Vec::new();
    let config = Config::load_with_notices(tmp.path(), |notice| {
        if let ConfigNotice::Precedence {
            winner, shadowed, ..
        } = notice
        {
            precedence.push((winner, shadowed));
        }
    })
    .expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    assert_eq!(
        precedence,
        [(ConfigForm::DotConfigProseToml, ConfigForm::PyprojectTable)]
    );
}

#[test]
fn load_prefers_prose_toml_over_sibling_dotconfig() {
    let tmp = TempDir::new().expect("tempdir");
    write_prose_toml(tmp.path(), "code-line-length = 120\n");
    write_dotconfig_prose_toml(tmp.path(), "code-line-length = 80\n");

    let mut precedence = Vec::new();
    let config = Config::load_with_notices(tmp.path(), |notice| {
        if let ConfigNotice::Precedence {
            winner, shadowed, ..
        } = notice
        {
            precedence.push((winner, shadowed));
        }
    })
    .expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    assert_eq!(
        precedence,
        [(ConfigForm::ProseToml, ConfigForm::DotConfigProseToml)]
    );
}

#[test]
fn load_prefers_prose_toml_over_sibling_pyproject() {
    let tmp = TempDir::new().expect("tempdir");
    write_prose_toml(tmp.path(), "code-line-length = 120\n");
    write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");

    let config = Config::load(tmp.path()).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn load_reads_dotconfig_prose_toml() {
    let tmp = TempDir::new().expect("tempdir");
    write_dotconfig_prose_toml(tmp.path(), "code-line-length = 120\n");

    let config = Config::load(tmp.path()).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn load_reads_pure_prose_toml() {
    let tmp = TempDir::new().expect("tempdir");
    write_prose_toml(tmp.path(), "code-line-length = 120\n");

    let config = Config::load(tmp.path()).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn load_unknown_key_invokes_callback() {
    let tmp = TempDir::new().expect("tempdir");
    write_pyproject(
        tmp.path(),
        "[tool.prose]\nunknown-future-key = \"whatever\"\n",
    );

    let mut captured = Vec::new();
    let config = Config::load_with_notices(tmp.path(), |notice| {
        if let ConfigNotice::UnknownKey(key) = notice {
            captured.push(key.to_owned());
        }
    })
    .expect("loads");

    assert_eq!(captured, ["unknown-future-key"]);
    assert!(config.rules.align_equals.enabled);
}

#[test]
fn load_unknown_key_routes_through_default_warn_callback() {
    let tmp = TempDir::new().expect("tempdir");
    write_pyproject(tmp.path(), "[tool.prose]\nunknown-future-key = \"x\"\n");

    let config = Config::load(tmp.path()).expect("loads");

    assert!(config.rules.align_equals.enabled);
}

#[test]
fn load_unreadable_config_returns_io_error() {
    let tmp = TempDir::new().expect("tempdir");
    std::fs::create_dir(tmp.path().join("prose.toml")).expect("dir at config path");

    assert_matches!(Config::load(tmp.path()), Err(ConfigError::Io(_)));
}

#[test]
fn load_walks_past_sectionless_pyproject_to_ancestor_prose_toml() {
    let tmp = TempDir::new().expect("tempdir");
    let nested = tmp.path().join("child");
    std::fs::create_dir_all(&nested).expect("nested dirs create");
    write_pyproject(&nested, "[project]\nname = \"x\"\n");
    write_prose_toml(tmp.path(), "code-line-length = 120\n");

    let config = Config::load(&nested).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn load_walks_up_to_ancestor_directory() {
    let tmp = TempDir::new().expect("tempdir");
    let nested = tmp.path().join("a/b/c");
    std::fs::create_dir_all(&nested).expect("nested dirs create");
    write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");

    let config = Config::load(&nested).expect("loads");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
}

#[test]
fn rules_unknown_subtable_key_invokes_notice() {
    let tmp = TempDir::new().expect("tempdir");
    write_pyproject(
        tmp.path(),
        "[tool.prose.rules.align-equals]\nbogus-knob = 1\n",
    );

    let mut captured = Vec::new();
    Config::load_with_notices(tmp.path(), |notice| {
        if let ConfigNotice::UnknownKey(key) = notice {
            captured.push(key.to_owned());
        }
    })
    .expect("loads");

    assert_eq!(captured, ["rules.align-equals.bogus-knob"]);
}
