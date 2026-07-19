//! Class-body member classifiers. Reports the method group and names
//! the decorator condition that pins a method run.

use ruff_python_ast::{Stmt, StmtFunctionDef, helpers::is_dunder};

use crate::primitives::{
    binding::ann_assign_with_named_field, decorator::decorator_simple_name,
    params::pins_positional_params,
};

/// True when a class body has at least two `Stmt::AnnAssign` field
/// declarations and at least one method whose decorator carries
/// positional arguments.
pub(super) fn class_pins_methods(body: &[Stmt]) -> bool {
    body.iter()
        .filter(|s| ann_assign_with_named_field(s).is_some())
        .nth(1)
        .is_some()
        && body
            .iter()
            .filter_map(Stmt::as_function_def_stmt)
            .any(pins_positional_params)
}

/// Returns the method-group index. `0` for dunders, `1` for
/// `@property` / `@cached_property` (decided by the first decorator),
/// `2` for single-leading-underscore privates, `3` for public.
pub(super) fn method_group(f: &StmtFunctionDef) -> u8 {
    let name = f.name.as_str();
    if is_dunder(name) {
        0
    } else if f
        .decorator_list
        .first()
        .and_then(decorator_simple_name)
        .is_some_and(|n| matches!(n, "cached_property" | "property"))
    {
        1
    } else if name.starts_with('_') {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_class, parse};

    #[rstest]
    #[case(
        "class C:\n    a: int\n    b: int\n    @route(path)\n    def run(self): pass\n",
        true
    )]
    #[case("class C:\n    a: int\n    b: int\n    def run(self): pass\n", false)]
    #[case(
        "class C:\n    a: int\n    @route(path)\n    def run(self): pass\n",
        false
    )]
    #[case(
        "class C:\n    a: int\n    b: int\n    @cache\n    def run(self): pass\n",
        false
    )]
    fn class_pins_methods_needs_two_fields_and_a_positional_decorator(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        assert_eq!(class_pins_methods(&first_class(&s).body), expected);
    }

    #[test]
    fn method_group_orders_dunder_property_private_public() {
        let src = indoc! {"
            class C:
                def __init__(self): pass
                @property
                def name(self): pass
                def _helper(self): pass
                def public(self): pass
        "};
        let s = parse(src);
        let class = first_class(&s);
        let groups: Vec<u8> = class
            .body
            .iter()
            .filter_map(Stmt::as_function_def_stmt)
            .map(method_group)
            .collect();
        assert_eq!(groups, vec![0, 1, 2, 3]);
    }
}
