//! Carries the slide over a tree, reaching the `Identifier` and
//! format-spec ranges the walk does not visit on its own.

use std::{cell::RefCell, ops::Range};

use ruff_python_ast::{
    self as ast, Alias, Arguments, BytesLiteral, Comprehension, Decorator, ExceptHandler, Expr,
    FString, Identifier, InterpolatedStringElement, Keyword, MatchCase, ModModule, Parameter,
    Parameters, Pattern, PatternArguments, PatternKeyword, Stmt, StringLiteral, TString, TypeParam,
    TypeParams, WithItem,
    visitor::transformer::{
        Transformer, walk_alias, walk_arguments, walk_bytes_literal, walk_comprehension,
        walk_decorator, walk_except_handler, walk_expr, walk_f_string,
        walk_interpolated_string_element, walk_keyword, walk_match_case, walk_parameter,
        walk_parameters, walk_pattern, walk_pattern_arguments, walk_pattern_keyword, walk_stmt,
        walk_string_literal, walk_t_string, walk_type_param, walk_type_params, walk_with_item,
    },
};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use super::deltas::Deltas;

/// Slides a node's own range, leaving the walk to reach its children.
macro_rules! slide_node {
    ($visit:ident, $walk:ident, $node:ty) => {
        fn $visit(&self, node: &mut $node) {
            node.range = self.slide(node.range);
            $walk(self, node);
        }
    };
    ($visit:ident, $walk:ident, $node:ty, $name:ident) => {
        fn $visit(&self, node: &mut $node) {
            node.range = self.slide(node.range);
            self.slide_name(&mut node.$name);
            $walk(self, node);
        }
    };
}

/// Rewrites each node's own range in place, leaving the walk to reach
/// its children.
macro_rules! slide_variants {
    ($node:expr, $slide:expr, $enum:ident, $($variant:ident),+ $(,)?) => {
        match $node {
            $($enum::$variant(inner) => inner.range = $slide(inner.range),)+
        }
    };
}

/// Carries [`Deltas`] over a tree, sliding every node and identifier
/// range it reaches, swapping in each freshly parsed statement where
/// the slid range meets its nested window, and replacing the module
/// body's statements inside each run window with the statements parsed
/// from it.
pub(super) struct Slide<'map> {
    deltas: &'map Deltas<'map>,
    grafts: RefCell<FxHashMap<TextRange, Stmt>>,
    run_spans: Vec<TextRange>,
    runs: RefCell<Vec<Vec<Stmt>>>,
}

impl<'map> Slide<'map> {
    /// Builds the pass, `grafts` pairing each nested window's held range
    /// with the statement parsed from it and `runs` pairing each module
    /// window's held range with the statements parsed from it.
    pub(super) fn new(
        deltas: &'map Deltas<'map>,
        grafts: impl IntoIterator<Item = (TextRange, Stmt)>,
        runs: impl IntoIterator<Item = (TextRange, Vec<Stmt>)>,
    ) -> Self {
        let mut runs: Vec<_> = runs.into_iter().collect();
        runs.sort_by_key(|(held, _)| held.start());
        let (run_spans, runs) = runs.into_iter().unzip();
        Self {
            deltas,
            grafts: RefCell::new(grafts.into_iter().collect()),
            run_spans,
            runs: RefCell::new(runs),
        }
    }

    /// The statement parsed for the nested window held at `range`,
    /// taken once.
    fn graft(&self, range: TextRange) -> Option<Stmt> {
        self.grafts.borrow_mut().remove(&range)
    }

    /// True where `range`, in the held buffer, lies inside a run window,
    /// whose statements the reparse replaces whole.
    fn inside_a_run(&self, range: TextRange) -> bool {
        self.run_spans.iter().any(|held| held.contains_range(range))
    }

    fn slide(&self, range: TextRange) -> TextRange {
        self.deltas.slide(range)
    }

    fn slide_name(&self, name: &mut Identifier) {
        self.slide_ranges([&mut name.range]);
    }

    fn slide_names<'node>(&self, names: impl IntoIterator<Item = &'node mut Identifier>) {
        self.slide_ranges(names.into_iter().map(|name| &mut name.range));
    }

    /// Slides each of `ranges`, the fields a node list carries that the
    /// walk does not reach on its own.
    fn slide_ranges<'node>(&self, ranges: impl IntoIterator<Item = &'node mut TextRange>) {
        for range in ranges {
            *range = self.slide(*range);
        }
    }

    /// Slides `module`'s own range and every range beneath it, grafting
    /// each nested window's statement in as the walk reaches it and
    /// each run window's statements in over the body slots the run
    /// held.
    pub(super) fn over_module(&self, module: &mut ModModule) {
        module.range = self.deltas.slide_window(module.range);
        let slots: Vec<Range<usize>> = self
            .run_spans
            .iter()
            .map(|held| {
                let from = module
                    .body
                    .partition_point(|stmt| stmt.start() < held.start());
                let to = module
                    .body
                    .partition_point(|stmt| stmt.start() < held.end());
                from..to
            })
            .collect();
        self.visit_body(&mut module.body);
        let runs = self.runs.take();
        for (slots, mut stmts) in slots.into_iter().zip(runs).rev() {
            if slots.len() == 1
                && stmts.len() == 1
                && let Some(stmt) = stmts.pop()
            {
                module.body[slots.start] = stmt;
                continue;
            }
            let tail: Vec<Stmt> = module.body.drain(slots.end..).collect();
            module.body.truncate(slots.start);
            module.body.extend(stmts);
            module.body.extend(tail);
        }
        debug_assert!(
            self.grafts.borrow().is_empty(),
            "every nested window came from a statement the slide reaches",
        );
    }
}

impl Transformer for Slide<'_> {
    fn visit_alias(&self, alias: &mut Alias) {
        alias.range = self.slide(alias.range);
        self.slide_name(&mut alias.name);
        self.slide_names(alias.asname.as_mut());
        walk_alias(self, alias);
    }

    slide_node!(visit_arguments, walk_arguments, Arguments);

    slide_node!(visit_bytes_literal, walk_bytes_literal, BytesLiteral);

    slide_node!(visit_comprehension, walk_comprehension, Comprehension);

    slide_node!(visit_decorator, walk_decorator, Decorator);

    /// Walks `body`, leaving every statement inside a run window to the
    /// graft that replaces it.
    fn visit_body(&self, body: &mut [Stmt]) {
        for stmt in body {
            if !self.inside_a_run(stmt.range()) {
                self.visit_stmt(stmt);
            }
        }
    }

    fn visit_except_handler(&self, except_handler: &mut ExceptHandler) {
        let ExceptHandler::ExceptHandler(handler) = except_handler;
        handler.range = self.slide(handler.range);
        self.slide_names(handler.name.as_mut());
        walk_except_handler(self, except_handler);
    }

    fn visit_expr(&self, expr: &mut Expr) {
        slide_variants!(
            &mut *expr,
            |range| self.slide(range),
            Expr,
            Attribute,
            Await,
            BinOp,
            BooleanLiteral,
            BoolOp,
            BytesLiteral,
            Call,
            Compare,
            Dict,
            DictComp,
            EllipsisLiteral,
            FString,
            Generator,
            If,
            IpyEscapeCommand,
            Lambda,
            List,
            ListComp,
            Named,
            Name,
            NoneLiteral,
            NumberLiteral,
            Set,
            SetComp,
            Slice,
            Starred,
            StringLiteral,
            Subscript,
            TString,
            Tuple,
            UnaryOp,
            Yield,
            YieldFrom,
        );
        if let Expr::Attribute(attribute) = &mut *expr {
            self.slide_name(&mut attribute.attr);
        }
        walk_expr(self, expr);
    }

    slide_node!(visit_f_string, walk_f_string, FString);

    fn visit_interpolated_string_element(&self, element: &mut InterpolatedStringElement) {
        slide_variants!(
            &mut *element,
            |range| self.slide(range),
            InterpolatedStringElement,
            Interpolation,
            Literal,
        );
        if let InterpolatedStringElement::Interpolation(interpolation) = &mut *element
            && let Some(format_spec) = &mut interpolation.format_spec
        {
            format_spec.range = self.slide(format_spec.range);
        }
        walk_interpolated_string_element(self, element);
    }

    fn visit_keyword(&self, keyword: &mut Keyword) {
        keyword.range = self.slide(keyword.range);
        self.slide_names(keyword.arg.as_mut());
        walk_keyword(self, keyword);
    }

    slide_node!(visit_match_case, walk_match_case, MatchCase);

    slide_node!(visit_parameter, walk_parameter, Parameter, name);

    fn visit_parameters(&self, parameters: &mut Parameters) {
        parameters.range = self.slide(parameters.range);
        self.slide_ranges(
            parameters
                .posonlyargs
                .iter_mut()
                .chain(&mut parameters.args)
                .chain(&mut parameters.kwonlyargs)
                .map(|arg| &mut arg.range),
        );
        walk_parameters(self, parameters);
    }

    fn visit_pattern(&self, pattern: &mut Pattern) {
        slide_variants!(
            &mut *pattern,
            |range| self.slide(range),
            Pattern,
            MatchAs,
            MatchClass,
            MatchMapping,
            MatchOr,
            MatchSequence,
            MatchSingleton,
            MatchStar,
            MatchValue,
        );
        if let Pattern::MatchAs(ast::PatternMatchAs { name, .. })
        | Pattern::MatchStar(ast::PatternMatchStar { name, .. })
        | Pattern::MatchMapping(ast::PatternMatchMapping { rest: name, .. }) = &mut *pattern
        {
            self.slide_names(name.as_mut());
        }
        walk_pattern(self, pattern);
    }

    slide_node!(
        visit_pattern_arguments,
        walk_pattern_arguments,
        PatternArguments
    );

    slide_node!(
        visit_pattern_keyword,
        walk_pattern_keyword,
        PatternKeyword,
        attr
    );

    fn visit_stmt(&self, stmt: &mut Stmt) {
        if self.deltas.holds_still(stmt.range()) {
            return;
        }
        if let Some(parsed) = self.graft(stmt.range()) {
            *stmt = parsed;
            return;
        }
        slide_variants!(
            &mut *stmt,
            |range| self.slide(range),
            Stmt,
            AnnAssign,
            Assert,
            Assign,
            AugAssign,
            Break,
            ClassDef,
            Continue,
            Delete,
            Expr,
            For,
            FunctionDef,
            Global,
            If,
            Import,
            ImportFrom,
            IpyEscapeCommand,
            Match,
            Nonlocal,
            Pass,
            Raise,
            Return,
            Try,
            TypeAlias,
            While,
            With,
        );
        match &mut *stmt {
            Stmt::ClassDef(ast::StmtClassDef { name, .. })
            | Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => self.slide_name(name),
            Stmt::Global(ast::StmtGlobal { names, .. })
            | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => self.slide_names(names),
            Stmt::If(ast::StmtIf {
                elif_else_clauses, ..
            }) => self.slide_ranges(elif_else_clauses.iter_mut().map(|clause| &mut clause.range)),
            Stmt::ImportFrom(ast::StmtImportFrom { module, .. }) => {
                self.slide_names(module.as_mut());
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }

    slide_node!(visit_string_literal, walk_string_literal, StringLiteral);

    slide_node!(visit_t_string, walk_t_string, TString);

    fn visit_type_param(&self, type_param: &mut TypeParam) {
        slide_variants!(
            &mut *type_param,
            |range| self.slide(range),
            TypeParam,
            ParamSpec,
            TypeVar,
            TypeVarTuple,
        );
        let (TypeParam::ParamSpec(ast::TypeParamParamSpec { name, .. })
        | TypeParam::TypeVar(ast::TypeParamTypeVar { name, .. })
        | TypeParam::TypeVarTuple(ast::TypeParamTypeVarTuple { name, .. })) = &mut *type_param;
        self.slide_name(name);
        walk_type_param(self, type_param);
    }

    slide_node!(visit_type_params, walk_type_params, TypeParams);

    slide_node!(visit_with_item, walk_with_item, WithItem);
}
