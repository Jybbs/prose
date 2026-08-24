//! Closes a fractured argument list back onto one row. A break the
//! author hand-wrapped closes up, whereas a flush column, a list
//! carrying a comment, and one past the argument cap each hold, so a
//! rule measuring a construct reads the width layout settles on.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, ArgOrKeyword, Arguments, Expr, ExprCall,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::ReflowCallsConfig,
    primitives::{
        call_keywords::{CallTargets, takes_keyword_form},
        edit::apply_inline_edits,
        layout::is_fractured,
    },
    source::Source,
};

mod join;

pub(crate) use join::outermost;

use join::join_edits;

/// The joins closing every fractured argument list beneath one
/// expression, ascending by start and disjoint, read back per range.
pub(crate) struct Joins(Vec<Edit>);

impl Joins {
    /// `range`'s text with every join inside it applied.
    pub(crate) fn settled<'s>(&self, source: &'s Source, range: TextRange) -> Cow<'s, str> {
        apply_inline_edits(source, range, &self.0)
    }
}

/// The terms a fracture closes under, resolved from configuration.
/// `cap` is the argument count past which a list keeps its break, and
/// `closes` is clear where `reflow_calls` is off and no fracture shuts
/// at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settings<'a> {
    cap: Option<usize>,
    closes: bool,
    targets: Option<&'a CallTargets<'a>>,
}

impl Settings<'_> {
    /// These settings resolving a call against `targets`, the map
    /// [`module_call_params`] builds for one source, so the join reads
    /// the count trigger the way `reflow-calls` explodes it.
    pub(crate) fn against<'t>(self, targets: &'t CallTargets<'t>) -> Settings<'t> {
        Settings {
            cap: self.cap,
            closes: self.closes,
            targets: Some(targets),
        }
    }

    /// True where `reflow-calls` runs at all, so a list its length
    /// trigger reaches explodes once the rule takes its turn.
    pub(crate) fn closes(self) -> bool {
        self.closes
    }

    /// True where `reflow-calls`'s count trigger explodes `call`, its
    /// arguments past the cap and every one taking keyword form.
    pub(crate) fn explodes(self, source: &Source, call: &ExprCall) -> bool {
        self.over_cap(call.arguments.len()) && takes_keyword_form(source, call, self.targets)
    }

    /// The joins closing every fractured argument list beneath `expr`.
    pub(crate) fn joins(self, source: &Source, expr: &Expr) -> Joins {
        if !self.closes {
            return Joins(Vec::new());
        }
        Joins(join_edits(source, self, expr))
    }

    /// True where a list of `count` arguments sits past the cap a
    /// closing fracture holds to, the list `reflow-calls` explodes on its
    /// count trigger. False throughout where `reflow-calls` is off.
    fn over_cap(self, count: usize) -> bool {
        self.closes && self.cap.is_some_and(|cap| count > cap)
    }
}

impl From<&ReflowCallsConfig> for Settings<'_> {
    fn from(rules: &ReflowCallsConfig) -> Self {
        Self {
            cap: rules.max_args.cap(),
            closes: rules.enabled,
            targets: None,
        }
    }
}
