//! Resolves a name to the alias target it stands for.

use rustc_hash::FxHashSet;

use super::*;

/// One classification pass, holding the bases already resolved so a
/// cyclic base settles once.
pub(super) struct Resolver<'ctx, 'src> {
    pub(super) ctx: &'ctx AliasContext<'src>,
    pub(super) visited: FxHashSet<&'src str>,
}

impl<'src> Resolver<'_, 'src> {
    pub(super) fn alias_value(&mut self, value: &Expr) -> bool {
        match value {
            Expr::BinOp(ExprBinOp {
                left,
                op: Operator::BitOr,
                right,
                ..
            }) => self.union_arm(left) && self.union_arm(right),
            Expr::Subscript(subscript) => !self.subscript_is_runtime(subscript),
            _ => UnqualifiedName::from_expr(value).is_some_and(|base| self.dotted_is_alias(&base)),
        }
    }

    /// True when a dotted name may hold a type. A dunder anywhere along
    /// the path names a runtime value (`__builtins__`, `sys.__stdout__`),
    /// and a head the module binds to data holds data.
    fn dotted_is_alias(&mut self, base: &UnqualifiedName<'_>) -> bool {
        !base.segments().iter().any(|segment| is_dunder(segment))
            && !base
                .segments()
                .first()
                .copied()
                .is_some_and(|head| self.name_binds_data(head))
    }

    /// True when `arm` is a type a PEP 604 union may carry, an alias
    /// value or the `None` of an optional. A bare `None` outside a union
    /// binds a sentinel rather than naming a type.
    fn union_arm(&mut self, arm: &Expr) -> bool {
        arm.is_none_literal_expr() || self.alias_value(arm)
    }

    /// True when `subscript` indexes a value at runtime rather than
    /// parametrizing a type, proven either by the module binding the base
    /// to data or by a node the typing grammar never admits in the slice.
    fn subscript_is_runtime(&mut self, subscript: &ExprSubscript) -> bool {
        let Some(base) = UnqualifiedName::from_expr(&subscript.value) else {
            // A call or a chained subscript roots the base, as in
            // `get_registry().default` and `registry[0][1]`.
            return true;
        };
        if !self.dotted_is_alias(&base) {
            return true;
        }
        let slice = subscript.slice.as_ref();
        match base.segments().last().copied() {
            // PEP 586 admits an int, a bool, and bytes, and `Literal` is
            // the sole construct that carries them.
            Some(tail) if is_literal_member(tail) => self.runtime_only(slice, true),
            // PEP 593 leaves every argument after the first arbitrary, so
            // `Annotated[int, Field(gt=0)]` reads only its type.
            Some(tail) if is_pep_593_generic_member(tail) => {
                let annotated = slice
                    .as_tuple_expr()
                    .and_then(|tuple| tuple.elts.first())
                    .unwrap_or(slice);
                self.runtime_only(annotated, false)
            }
            _ => self.runtime_only(slice, false),
        }
    }

    /// True when `expr` cannot stand at this position in a type
    /// expression, which proves the enclosing subscript indexes data.
    /// `in_literal` widens the grammar to the arguments a `Literal[...]`
    /// carries.
    ///
    /// The match is exhaustive over `Expr` rather than closing on a
    /// wildcard, so a new AST node raises a compile error instead of
    /// silently reading as a type.
    fn runtime_only(&mut self, expr: &Expr, in_literal: bool) -> bool {
        match expr {
            // No type expression admits any of these.
            Expr::Await(_)
            | Expr::BoolOp(_)
            | Expr::Call(_)
            | Expr::Compare(_)
            | Expr::Dict(_)
            | Expr::DictComp(_)
            | Expr::FString(_)
            | Expr::Generator(_)
            | Expr::If(_)
            | Expr::IpyEscapeCommand(_)
            | Expr::Lambda(_)
            | Expr::ListComp(_)
            | Expr::Named(_)
            | Expr::Set(_)
            | Expr::SetComp(_)
            | Expr::Slice(_)
            | Expr::TString(_)
            | Expr::Yield(_)
            | Expr::YieldFrom(_) => true,

            // Only the PEP 604 union, and only when both arms hold types.
            Expr::BinOp(ExprBinOp {
                left, op, right, ..
            }) => {
                *op != Operator::BitOr
                    || self.runtime_only(left, in_literal)
                    || self.runtime_only(right, in_literal)
            }

            // A signed integer stands only inside `Literal`, so
            // `items[-1]` indexes data whereas `Literal[-1]` names a type.
            Expr::UnaryOp(ExprUnaryOp { op, operand, .. }) => {
                !in_literal
                    || !matches!(
                        (op, operand.as_ref()),
                        (
                            UnaryOp::UAdd | UnaryOp::USub,
                            Expr::NumberLiteral(ExprNumberLiteral {
                                value: Number::Int(_),
                                ..
                            })
                        )
                    )
            }

            // An int, a bool, and bytes ride on `Literal` alone.
            Expr::BooleanLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::NumberLiteral(ExprNumberLiteral {
                value: Number::Int(_),
                ..
            }) => !in_literal,
            // A float and a complex have no place in any type expression.
            Expr::NumberLiteral(_) => true,

            // A dunder names a runtime value, and a name the module binds
            // to data indexes it (`table[idx]` under `idx = 0`).
            Expr::Name(name) => is_dunder(&name.id) || self.name_binds_data(&name.id),
            Expr::Attribute(ExprAttribute { attr, value, .. }) => {
                is_dunder(attr) || self.runtime_only(value, in_literal)
            }

            // A container inherits the proof its members carry.
            Expr::List(ExprList { elts, .. }) | Expr::Tuple(ExprTuple { elts, .. }) => elts
                .iter()
                .any(|element| self.runtime_only(element, in_literal)),
            Expr::Starred(starred) => self.runtime_only(&starred.value, in_literal),
            Expr::Subscript(subscript) => self.subscript_is_runtime(subscript),

            // A string is a forward reference, `None` is a type, and `...`
            // is the shape of `Callable[..., int]`.
            Expr::EllipsisLiteral(_) | Expr::NoneLiteral(_) | Expr::StringLiteral(_) => false,
        }
    }

    /// True when this module binds `name` to data it builds. Anything the
    /// module leaves undecided reads as false, in that only a value built
    /// here proves the name holds data.
    fn name_binds_data(&mut self, name: &str) -> bool {
        let analysis = self.ctx.analysis;
        let kinds = analysis.module_binding_kinds(name);
        // An unbound name is a builtin, a star-import, or a global
        // injected at runtime. A `class` statement binds a type. An
        // import and a rebind are decided elsewhere. None says data.
        if kinds.is_empty()
            || kinds.contains(&BindingKind::ClassDef)
            || kinds.contains(&BindingKind::Import)
            || analysis.module_reassigned(name)
        {
            return false;
        }
        let Some((&key, &value)) = self.ctx.values.get_key_value(name) else {
            return false;
        };
        // A cycle, as in `A = A[0]`.
        if !self.visited.insert(key) {
            return false;
        }
        // A call binds a type as readily as data (`T = TypeVar("T")`),
        // and an attribute is undecidable in the module alone
        // (`opener = TarFile.open` against `path_sep = os.sep`).
        !matches!(value, Expr::Attribute(_) | Expr::Call(_)) && !self.alias_value(value)
    }
}
