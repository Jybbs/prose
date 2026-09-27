//! Tests for how far a run reaches a module it could not compare, covering
//! the import a failed run named, the exception it raised, the module's path,
//! and the word a report spells for each.

use rstest::rstest;

use super::*;
use crate::reach::Reach;

#[rstest]
#[case::a_sibling("pkg/mod.py", "pkg.other")]
#[case::a_prefix_sharing_package("pkgx/mod.py", "pkg")]
#[case::another_package("encodings/mbcs.py", "codecs")]
fn a_reach_leaves_an_import_naming_another_module_to_the_exception(
    #[case] relative: &str,
    #[case] importing: &str,
) {
    assert_eq!(
        Reach::of(relative, &raising("ImportError", Some(importing))),
        Reach::Module
    );
}

#[rstest]
#[case::itself("pkg/mod.py", "ModuleNotFoundError", "pkg.mod")]
#[case::its_package("pkg/mod.py", "ImportError", "pkg")]
#[case::a_top_level_module("os.py", "ImportError", "os")]
#[case::a_package_init("pkg/__init__.py", "ImportError", "pkg")]
#[case::an_enclosing_package("pkg/sub/mod.py", "ImportError", "pkg")]
#[case::a_vendored_distribution(
    "site-packages/pip/_internal/cache.py",
    "ModuleNotFoundError",
    "pip"
)]
fn a_reach_reads_an_import_naming_the_module_or_a_package_holding_it_as_the_loader(
    #[case] relative: &str,
    #[case] raised: &str,
    #[case] importing: &str,
) {
    assert_eq!(
        Reach::of(relative, &raising(raised, Some(importing))),
        Reach::Loader
    );
}

#[rstest]
#[case("dbm/gnu.py", "ModuleNotFoundError", Reach::Absent)]
#[case(
    "multiprocessing/popen_spawn_win32.py",
    "ModuleNotFoundError",
    Reach::Absent
)]
#[case("asyncio/windows_events.py", "ImportError", Reach::Platform)]
#[case("pip/_vendor/truststore/_windows.py", "ImportError", Reach::Platform)]
#[case(
    "pip/_vendor/urllib3/contrib/emscripten/connection.py",
    "ImportError",
    Reach::Platform
)]
#[case("encodings/mbcs.py", "ImportError", Reach::Module)]
#[case("pip/__pip-runner__.py", "AssertionError", Reach::Module)]
fn a_reach_reads_the_exception_then_the_path(
    #[case] relative: &str,
    #[case] raised: &str,
    #[case] want: Reach,
) {
    assert_eq!(Reach::of(relative, &raising(raised, None)), want);
}

#[rstest]
#[case(Reach::Absent, "absent")]
#[case(Reach::Loader, "loader")]
#[case(Reach::Module, "module")]
#[case(Reach::Platform, "platform")]
fn a_reach_spells_one_word_for_the_report(#[case] reach: Reach, #[case] spelt: &str) {
    assert_eq!(reach.to_string(), spelt);
}
