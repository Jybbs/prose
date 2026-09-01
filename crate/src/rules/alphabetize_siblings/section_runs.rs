//! One section's permutation runs, each tiered once for the whole
//! fixed-point loop, and the split a fenced slot puts through a section
//! so no permutation seats a member across one.

use std::{iter, ops::Range};

use itertools::Itertools;
use ruff_python_ast::Stmt;
use ruff_text_size::TextSize;

use super::{
    class_graph::ClassAssigns,
    members::{class_pins_methods, function_key},
    module_graph::{ModuleDefs, module_def_run, permute_module_run},
};
use crate::primitives::tiering::{DefRun, Evaluation};

/// One section's permutation runs, each tiered once for the whole
/// fixed-point loop. A class body keeps a run per family, the method sort
/// carrying its own pinned-field gate, whereas module scope sorts its
/// classes and functions as one banded run.
pub(super) struct SectionRuns<'a, 'src> {
    assigns: Option<ClassAssigns<'a, 'src>>,
    classes: Option<DefRun<'a, 'src, &'src str>>,
    functions: Option<DefRun<'a, 'src, (u8, &'src str)>>,
    modules: Option<ModuleDefs<'a, 'src>>,
}

impl<'a, 'src> SectionRuns<'a, 'src> {
    /// Prepares whichever runs `scope` and the sort flags enable over
    /// `section`, leaving every other one `None`.
    pub(super) fn of(
        body: &'src [Stmt],
        section: Range<usize>,
        evaluation: Evaluation<'a, 'src>,
        in_class: bool,
        group_methods: bool,
        orders_members: bool,
        sort_definitions: bool,
    ) -> Self {
        if !in_class {
            return Self {
                assigns: None,
                classes: None,
                functions: None,
                modules: sort_definitions
                    .then(|| module_def_run(body, section, evaluation, group_methods))
                    .flatten(),
            };
        }
        let sorts_methods = sort_definitions && !class_pins_methods(&body[section.clone()]);
        Self {
            assigns: (!orders_members)
                .then(|| ClassAssigns::of(body, section.clone(), evaluation))
                .flatten(),
            classes: sort_definitions
                .then(|| {
                    DefRun::of(body, section.clone(), evaluation, |s| {
                        s.as_class_def_stmt().map(|c| {
                            let name = c.name.as_str();
                            (name, name)
                        })
                    })
                })
                .flatten(),
            functions: sorts_methods
                .then(|| {
                    DefRun::of(body, section, evaluation, |s| {
                        s.as_function_def_stmt()
                            .map(|f| (f.name.as_str(), function_key(f, group_methods)))
                    })
                })
                .flatten(),
            modules: None,
        }
    }

    /// Permutes each prepared run of this section against `order`, in the
    /// order the families settle.
    pub(super) fn permute(
        &self,
        order: &mut [usize],
        body: &'src [Stmt],
        holds: impl Fn(&'src Stmt) -> bool + Copy,
        keyword_fields_from: TextSize,
    ) {
        if let Some(run) = &self.classes {
            run.permute(order, body, holds, |tier, key| (tier, key));
        }
        if let Some(run) = &self.assigns {
            run.permute(order, body, keyword_fields_from);
        }
        if let Some(run) = &self.functions {
            run.permute(order, body, holds, |tier, key| (tier, key));
        }
        if let Some(run) = &self.modules {
            permute_module_run(run, order, body, holds);
        }
    }
}

/// The `section` split ahead of every fenced slot inside it, so no
/// permutation seats a member across one. `fences` is in slot order.
pub(super) fn fenced_runs(section: &Range<usize>, fences: &[usize]) -> Vec<Range<usize>> {
    iter::once(section.start)
        .chain(
            fences
                .iter()
                .copied()
                .filter(|fence| section.contains(fence) && *fence > section.start),
        )
        .chain(iter::once(section.end))
        .tuple_windows()
        .map(|(start, end)| start..end)
        .collect()
}
