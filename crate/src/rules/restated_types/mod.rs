//! Reports a docstring entry whose parenthesized type group restates an
//! annotation the code already carries. A parameter-documenting section
//! resolves each entry name against the enclosing function's
//! parameters, variadics included, and an `Attributes:` section against
//! the class body's annotated fields. The report carries no edit.

use ruff_python_ast::Stmt;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{
        binding::ann_assign_with_named_field,
        docstring::{documented_definitions, entry_carrying_sections},
    },
    rules::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct RestatedTypes;

impl RestatedTypes {
    pub(crate) const MESSAGE: &'static str =
        "Flag a docstring type group the code already annotates";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for RestatedTypes {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let rule = self.id();
        let mut diagnostics = Vec::new();
        for (definition, lit) in documented_definitions(source) {
            for section in entry_carrying_sections(source, lit) {
                let Some(documented) = Documented::from_heading(section.heading) else {
                    continue;
                };
                diagnostics.extend(section.entries.iter().filter_map(|entry| {
                    let group = entry.type_group?;
                    documented.annotates(definition, entry.name).then(|| {
                        Diagnostic::lint(
                            rule,
                            group,
                            format!(
                                "`{}` restates a type the {} already annotates",
                                entry.name,
                                documented.declarer(),
                            ),
                        )
                    })
                }));
            }
        }
        diagnostics
    }
}

/// The member set a Google-style section's entries name.
#[derive(Clone, Copy, Debug)]
enum Documented {
    Attributes,
    Parameters,
}

impl Documented {
    /// The member set the section headed `heading` documents, read
    /// without its trailing `:`. `None` for a heading naming neither
    /// set, covering `Returns:` and `Raises:` alike.
    fn from_heading(heading: &str) -> Option<Self> {
        match heading {
            "Args" | "Arguments" | "Keyword Args" | "Keyword Arguments" | "Other Args"
            | "Other Arguments" | "Other Params" | "Other Parameters" | "Parameters" => {
                Some(Self::Parameters)
            }
            "Attributes" => Some(Self::Attributes),
            _ => None,
        }
    }

    /// True when `definition` declares `name` as an annotated member of
    /// this set. False where the definition kind and the member set
    /// disagree, covering an `Attributes:` section on a function and a
    /// parameter section on a class.
    fn annotates(self, definition: &Stmt, name: &str) -> bool {
        match (self, definition) {
            (Self::Attributes, Stmt::ClassDef(class)) => class
                .body
                .iter()
                .filter_map(ann_assign_with_named_field)
                .any(|(_, field)| field == name),
            (Self::Parameters, Stmt::FunctionDef(function)) => function
                .parameters
                .iter()
                .any(|param| param.name().as_str() == name && param.annotation().is_some()),
            _ => false,
        }
    }

    /// The construct a report names as carrying the annotation.
    fn declarer(self) -> &'static str {
        match self {
            Self::Attributes => "class body",
            Self::Parameters => "signature",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;
    use crate::{diagnostics::Severity, testing::parse};

    #[test]
    fn a_report_carries_no_fix() {
        let src = "def dial(host: str):\n    \"\"\"\n    Summary.\n\n    Args:\n        host (str): The remote.\n    \"\"\"\n";
        let diagnostics = RestatedTypes::from_config(&Config::default()).lint(&parse(src));

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Lint);
        assert!(diagnostics[0].fix.is_none());
    }

    #[rstest]
    fn from_heading_reads_every_parameter_documenting_heading(
        #[values(
            "Args",
            "Arguments",
            "Keyword Args",
            "Keyword Arguments",
            "Other Args",
            "Other Arguments",
            "Other Params",
            "Other Parameters",
            "Parameters"
        )]
        heading: &str,
    ) {
        assert_matches!(
            Documented::from_heading(heading),
            Some(Documented::Parameters),
        );
    }

    #[test]
    fn from_heading_reads_the_attributes_heading() {
        assert_matches!(
            Documented::from_heading("Attributes"),
            Some(Documented::Attributes),
        );
    }

    #[rstest]
    fn from_heading_rejects_a_heading_documenting_neither_set(
        #[values("Returns", "Raises", "Yields", "Examples", "Note", "Steps", "args")] heading: &str,
    ) {
        assert!(Documented::from_heading(heading).is_none());
    }
}
