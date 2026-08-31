//! Carries the slide over a tree, reaching the `Identifier` and
//! format-spec ranges the walk does not visit on its own.

use std::cell::RefCell;

use rustc_hash::FxHashMap;

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
/// range it reaches and swapping in each freshly parsed statement where
/// the slid range meets its window.
pub(super) struct Slide<'map> {
    deltas: &'map Deltas<'map>,
    grafts: RefCell<FxHashMap<TextRange, Stmt>>,
}

impl<'map> Slide<'map> {
    /// Builds the pass, `grafts` pairing each window's slid range with
    /// the statement parsed from it.
    pub(super) fn new(
        deltas: &'map Deltas<'map>,
        grafts: impl IntoIterator<Item = (TextRange, Stmt)>,
    ) -> Self {
        Self {
            deltas,
            grafts: RefCell::new(grafts.into_iter().collect()),
        }
    }

    /// The statement parsed for the window at `range`, taken once.
    fn graft(&self, range: TextRange) -> Option<Stmt> {
        self.grafts.borrow_mut().remove(&range)
    }

    fn slide(&self, range: TextRange) -> TextRange {
        self.deltas.slide(range)
    }

    fn slide_name(&self, name: &mut Identifier) {
        name.range = self.slide(name.range);
    }

    fn slide_names<'node>(&self, names: impl IntoIterator<Item = &'node mut Identifier>) {
        for name in names {
            self.slide_name(name);
        }
    }

    /// Slides `module`'s own range and every range beneath it, grafting
    /// each window's statement in as the walk reaches it.
    pub(super) fn over_module(&self, module: &mut ModModule) {
        module.range = self.deltas.slide(module.range);
        for stmt in &mut module.body {
            self.visit_stmt(stmt);
        }
        debug_assert!(
            self.grafts.borrow().is_empty(),
            "every window came from a statement the slide reaches",
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
        for arg in parameters
            .posonlyargs
            .iter_mut()
            .chain(&mut parameters.args)
            .chain(&mut parameters.kwonlyargs)
        {
            arg.range = self.slide(arg.range);
        }
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
        match &mut *pattern {
            Pattern::MatchAs(ast::PatternMatchAs { name, .. })
            | Pattern::MatchStar(ast::PatternMatchStar { name, .. })
            | Pattern::MatchMapping(ast::PatternMatchMapping { rest: name, .. }) => {
                self.slide_names(name.as_mut());
            }
            _ => {}
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
        if let Some(parsed) = self.graft(stmt.range()) {
            *stmt = parsed;
            return;
        }
        match &mut *stmt {
            Stmt::ClassDef(ast::StmtClassDef { name, .. })
            | Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => self.slide_name(name),
            Stmt::Global(ast::StmtGlobal { names, .. })
            | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => self.slide_names(names),
            Stmt::If(ast::StmtIf {
                elif_else_clauses, ..
            }) => {
                for clause in elif_else_clauses {
                    clause.range = self.slide(clause.range);
                }
            }
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
        match &mut *type_param {
            TypeParam::ParamSpec(ast::TypeParamParamSpec { name, .. })
            | TypeParam::TypeVar(ast::TypeParamTypeVar { name, .. })
            | TypeParam::TypeVarTuple(ast::TypeParamTypeVarTuple { name, .. }) => {
                self.slide_name(name);
            }
        }
        walk_type_param(self, type_param);
    }

    slide_node!(visit_type_params, walk_type_params, TypeParams);

    slide_node!(visit_with_item, walk_with_item, WithItem);
}
