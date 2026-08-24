//! Lays out an implicitly concatenated string run one literal per
//! line. A single-line run breaks once its column carries it past
//! `code_line_length`, a run spanning lines with two parts sharing one
//! breaks whatever its width, and a run already one per line stays as
//! written, so the rule only ever breaks. The run wraps in parentheses
//! where no bracket carries it and breaks in place where one does, and
//! a docstring-slot run, a commented span, and a part with its own
//! line break each stay as written.

mod layout;

use layout::Layout;
pub(crate) use layout::concatenated_run;

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, StringLike};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        docstring::docstring_slots,
        edit::{narrowed_replacement, singleton_groups},
        layout::{Separator, explode_parens},
        orderer::any_sibling_shares_line,
        reserve,
        tokens::{is_closer, is_opener},
        walk::{Descent, ParentedProbe, walk_parented_exprs},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct StackAdjacentStrings {
    code_line_length: usize,
    reservations: reserve::Reservations,
}

impl StackAdjacentStrings {
    pub(crate) const MESSAGE: &'static str =
        "stack an implicitly concatenated string run one literal per line";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            reservations: config.equals_reservations(),
        }
    }
}

impl Rule for StackAdjacentStrings {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut layout = Layout {
            code_line_length: self.code_line_length,
            docstrings: docstring_slots(&source.ast().body),
            edits: Vec::new(),
            newline: source.newline_str(),
            reservations: source.columns(self.reservations),
            source,
        };
        walk_parented_exprs(source.ast(), &mut layout);
        singleton_groups(layout.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}
