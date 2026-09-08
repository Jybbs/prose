//! Prunes an import binding that adds nothing, covering a name the
//! module never references under `drop-unreferenced` and a repeat of a
//! binding an earlier import already made under `drop-duplicates`, one
//! walk deciding both facets so a repeat and the binding its drop
//! leaves unreferenced go together. A package `__init__.py` or its stub
//! reports an unreferenced binding rather than dropping it, as does a
//! module that writes no `__all__` and binds no name of its own.
//! `from __future__ import annotations` drops behind the
//! annotation analysis in `future`, and every other `__future__`
//! feature stays, as does a `from … import *`, a name `__all__` lists,
//! an import binding `__all__` itself, a name a second import rebinds
//! from another source, an `x as x` re-export alias, an import an
//! own-line comment leads, and every import in a module carrying a
//! file-level pragma naming the unused-import behavior.

use std::{ffi::OsStr, path::Path};

use ruff_diagnostics::Edit;
use ruff_python_ast::PythonVersion;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    rules::{Rule, RuleId, reflow_imports::Folds},
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
        "remove an import nothing references, or one that repeats an earlier import";

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

/// True when `source` is a package's `__init__.py` or its stub.
fn is_package_init(source: &Source) -> bool {
    matches!(
        Path::new(source.source_file().name())
            .file_name()
            .and_then(OsStr::to_str),
        Some("__init__.py" | "__init__.pyi")
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_python_ast::PySourceType;

    use super::*;
    use crate::{
        diagnostics::Severity,
        pipeline::Pipeline,
        testing::{applied_text, parse},
    };

    /// Parses `src` under a package `__init__.py` name.
    fn parse_init(src: &str) -> Source {
        Source::parse_named(src.to_owned(), "pkg/__init__.py").expect("test source parses")
    }

    /// Applies every fix group the rule plans over `source` and returns
    /// the resulting text.
    fn pruned_text(source: &Source) -> String {
        applied_text(source, rule().apply(source).concat())
    }

    fn rule() -> PruneInertImports {
        PruneInertImports::from_config(&Config::default())
    }

    #[test]
    fn a_noqa_head_naming_no_code_drops_its_unread_import() {
        let source = parse("# ruff: noqa\nvalue = 1\n\nimport json\n");

        assert_eq!(rule().apply(&source).len(), 1);
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn a_main_module_prunes_like_any_other_file() {
        let source = Source::parse_named(
            "import numpy as np\n\n__all__ = [\"value\"]\nvalue = 1\n".to_owned(),
            "pkg/__main__.py",
        )
        .expect("test source parses");

        assert_eq!(rule().apply(&source).len(), 1);
        assert!(rule().lint(&source).is_empty());
    }

    #[rstest]
    #[case::conditional_import_below_module_scope(
        "try:\n    import json\nexcept ImportError:\n    json = None\n"
    )]
    #[case::module_carrying_no_import("value = 1\n")]
    #[case::file_level_ruff_pragma("# ruff: noqa: F401\nimport json\n\nvalue = 1\n")]
    #[case::file_level_flake8_pragma("# flake8: noqa: E501, F401\nimport json\n\nvalue = 1\n")]
    #[case::file_level_pyright_pragma(
        "# pyright: reportUnusedImport=false\nimport json\n\nvalue = 1\n"
    )]
    fn a_module_the_rule_leaves_alone_neither_drops_nor_reports(#[case] src: &str) {
        let source = parse(src);

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
    fn a_package_init_reports_nothing_with_the_unreferenced_facet_off() {
        let mut config = Config::default();
        config.rules.prune_inert_imports.drop_unreferenced = false;
        let rule = PruneInertImports::from_config(&config);
        let source = parse_init("import json\n\nvalue = 1\n");

        assert!(rule.apply(&source).is_empty());
        assert!(rule.lint(&source).is_empty());
    }

    #[test]
    fn a_package_init_reports_rather_than_drops_where_it_declares_a_surface() {
        let source = parse_init("import json\n\n__all__ = [\"value\"]\nvalue = 1\n");

        assert!(rule().apply(&source).is_empty());
        assert_eq!(rule().lint(&source).len(), 1);
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
    fn a_quoted_annotation_holds_its_import_inside_a_package_init() {
        let source = parse_init("from typing import List\n\nx: \"List[int]\" = []\n");

        assert!(rule().apply(&source).is_empty());
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn a_repeat_drops_inside_a_package_init() {
        let source = parse_init("import os\nimport os\n\nvalue = os.getcwd()\n");

        assert_eq!(pruned_text(&source), "import os\n\nvalue = os.getcwd()\n");
    }

    #[test]
    fn a_shim_binding_no_name_of_its_own_reports_rather_than_drops() {
        let source = parse(
            "import shutil\nimport sys\n\ntry:\n    import ssl\nexcept ImportError:\n    ssl = None\n",
        );
        let diagnostics = rule().lint(&source);

        assert!(rule().apply(&source).is_empty());
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("writes no `__all__`"));
    }

    #[test]
    fn an_ignore_directive_silences_the_package_init_report() {
        let source = parse_init("import numpy as np  # prose: ignore[prune-inert-imports]\n");
        let pipeline =
            Pipeline::for_rule("prune-inert-imports", &Config::default()).expect("registered rule");

        let (_, diagnostics, _) = pipeline.run(source).expect("pipeline runs");

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn an_imported_dunder_all_holds_the_export_surface() {
        let source = parse("from io import SEEK_CUR, __all__\n\nvalue = SEEK_CUR\n");

        assert_eq!(
            pruned_text(&source),
            "from io import SEEK_CUR, __all__\n\nvalue = SEEK_CUR\n",
        );
    }

    #[test]
    fn an_unread_import_drops_where_the_module_binds_a_name_of_its_own() {
        let source = parse("import json\n\nvalue = 1\n");

        assert_eq!(pruned_text(&source), "\nvalue = 1\n");
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn an_unread_import_drops_whole_outside_a_package_init() {
        let source = parse("import json\n\n__all__ = [\"value\"]\nvalue = 1\n");

        assert_eq!(pruned_text(&source), "\n__all__ = [\"value\"]\nvalue = 1\n");
        assert!(rule().lint(&source).is_empty());
    }

    #[test]
    fn an_unread_repeat_reports_its_survivor_inside_a_package_init() {
        let source = parse_init("import os\nimport os\n");
        let diagnostics = rule().lint(&source);

        assert_eq!(pruned_text(&source), "import os\n");
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

        assert_eq!(pruned_text(&source), "import os\n\nvalue = os.getcwd()\n");
    }
}
