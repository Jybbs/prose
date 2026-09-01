//! Prunes an import binding that adds nothing, covering a name the
//! module never references under `drop-unreferenced` and a repeat of a
//! binding an earlier import already made under `drop-duplicates`, one
//! walk deciding both facets so a repeat and the binding its drop
//! leaves unreferenced go together. A package `__init__.py` or its stub
//! reports an unreferenced binding rather than dropping it, whereas a
//! repeat drops there too. `from __future__ import annotations` drops
//! behind the annotation analysis in `future`, and every other
//! `__future__` feature stays, as does a `from … import *`, a name
//! `__all__` lists, an import binding `__all__` itself, a name a second
//! import rebinds from another source, an `x as x` re-export alias, and
//! an import an own-line comment leads.

use std::{ffi::OsStr, path::Path};

use ruff_diagnostics::Edit;
use ruff_python_ast::PythonVersion;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    rule::{Rule, RuleId},
    rules::reflow_imports::Folds,
    source::Source,
};

mod annotations;
mod future;
mod inventory;
mod plan;
mod reexports;

use plan::Plan;

#[derive(Debug)]
pub(crate) struct PruneInertImports {
    duplicates: bool,
    folds: Folds,
    sorts_definitions: bool,
    target_version: Option<PythonVersion>,
    unreferenced: bool,
}

impl PruneInertImports {
    pub(crate) const MESSAGE: &'static str =
        "prune an import binding nothing references or a repeat of one already bound";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let facets = &config.rules.prune_inert_imports;
        Self {
            duplicates: facets.drop_duplicates,
            folds: Folds::from_config(config),
            sorts_definitions: config.alphabetize_siblings_enabled()
                && config.rules.alphabetize_siblings.sort_definitions,
            target_version: config.target_version,
            unreferenced: facets.drop_unreferenced,
        }
    }
}

impl Rule for PruneInertImports {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        Plan::of(self, source).edits(source)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        Plan::of(self, source).diagnostics(self.id())
    }
}

/// True when `source` is a package's `__init__.py` or its stub, whose
/// unread imports carry the package's re-export surface.
fn is_package_init(source: &Source) -> bool {
    Path::new(source.source_file().name())
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| matches!(name, "__init__.py" | "__init__.pyi"))
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PySourceType;

    use super::*;
    use crate::{
        diagnostics::Severity,
        pipeline::Pipeline,
        testing::{applied_text, parse},
    };

    /// Parses `src` under a package `__init__.py` name, which is what
    /// the re-export hold keys off.
    fn parse_init(src: &str) -> Source {
        Source::parse_named(src.to_owned(), "pkg/__init__.py").expect("test source parses")
    }

    fn rule() -> PruneInertImports {
        PruneInertImports::from_config(&Config::default())
    }

    #[test]
    fn a_conditional_import_below_module_scope_goes_unread() {
        let source = parse("try:\n    import json\nexcept ImportError:\n    json = None\n");
        assert!(rule().apply(&source).is_empty());
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn a_main_module_prunes_like_any_other_file() {
        let source = Source::parse_named(
            "import numpy as np\n\nvalue = 1\n".to_owned(),
            "pkg/__main__.py",
        )
        .expect("test source parses");

        assert_eq!(rule().apply(&source).len(), 1);
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn a_module_with_no_import_plans_nothing() {
        let source = parse("value = 1\n");
        assert!(rule().apply(&source).is_empty());
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn a_name_a_second_import_rebinds_holds_its_first_binding() {
        let source = parse(
            "from _pyimpl import filters\n\ntry:\n    from _cext import filters\nexcept ImportError:\n    pass\n",
        );

        assert!(rule().apply(&source).is_empty());
    }

    #[test]
    fn a_package_init_stub_holds_its_unread_imports() {
        let source = Source::build_module(
            "import numpy as np\n\nvalue = 1\n".to_owned(),
            "pkg/__init__.pyi",
            PySourceType::Stub,
        )
        .expect("test source parses");

        assert!(rule().apply(&source).is_empty());
        assert_eq!(rule().lint(&source).len(), 1);
    }

    #[test]
    fn a_repeat_drops_inside_a_package_init() {
        let source = parse_init("import os\nimport os\n\nvalue = os.getcwd()\n");
        let groups = rule().apply(&source);

        assert_eq!(
            applied_text(&source, groups.concat()),
            "import os\n\nvalue = os.getcwd()\n",
        );
    }

    #[test]
    fn an_ignore_directive_silences_the_package_init_report() {
        let source = parse_init("import numpy as np  # prose: ignore[prune-inert-imports]\n");
        let pipeline =
            Pipeline::for_rule("prune-inert-imports", &Config::default()).expect("registered rule");

        let (_, diagnostics) = pipeline.run(source).expect("pipeline runs");

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn an_imported_dunder_all_holds_the_export_surface() {
        let source = parse("from io import SEEK_CUR, __all__\n\nvalue = SEEK_CUR\n");
        let groups = rule().apply(&source);

        assert_eq!(
            applied_text(&source, groups.concat()),
            "from io import SEEK_CUR, __all__\n\nvalue = SEEK_CUR\n",
        );
    }

    #[test]
    fn an_unread_import_drops_whole_outside_a_package_init() {
        let source = parse("import json\n\nvalue = 1\n");
        let groups = rule().apply(&source);

        assert_eq!(applied_text(&source, groups.concat()), "\nvalue = 1\n");
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn an_unread_repeat_reports_its_survivor_inside_a_package_init() {
        let source = parse_init("import os\nimport os\n");
        let groups = rule().apply(&source);
        let diagnostics = rule().lint(&source);

        assert_eq!(applied_text(&source, groups.concat()), "import os\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.starts_with("`os` is imported"));
    }

    #[test]
    fn diagnostic_shape_pins_severity_no_fix_and_the_bound_name() {
        let source = parse_init("import numpy as np\n\nvalue = 1\n");
        let diagnostics = rule().lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert!(rule().apply(&source).is_empty());
        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert!(only.message.starts_with("`np` is imported"));
        assert!(only.message.contains("package's `__init__`"));
        assert_eq!(&source.text()[only.range], "numpy as np");
    }

    #[test]
    fn every_repeat_past_the_first_drops_in_one_group() {
        let source = parse("import os\nimport os\nimport os\n\nvalue = os.getcwd()\n");
        let groups = rule().apply(&source);
        let text = applied_text(&source, groups.concat());

        assert_eq!(text, "import os\n\nvalue = os.getcwd()\n");
    }
}
