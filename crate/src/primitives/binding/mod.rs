//! Per-`Source` binding-resolution table.
//!
//! Walks the module once and records, for every name introduced or
//! shadowed in a lexical scope, the offsets of every write and read,
//! a read finding no binding mid-walk resolving against the completed
//! scope chain after it. Consuming rules query by `BindingId`, name,
//! offset, or owning `&Stmt` rather than driving their own walk.
//!
//! ## Scope model
//!
//! Each scope is one of `Module`, `Function`, `Class`, or
//! `Comprehension`. The scope stack mirrors source-order nesting:
//! every `function-def`, `lambda`, `class-def`, and comprehension
//! pushes a frame, every comprehension's first generator iterable
//! evaluates in the enclosing scope, every walrus target lifts to
//! the nearest non-comprehension scope, and class-scope names are
//! invisible to nested functions and comprehensions.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use indexmap::IndexMap;
use ruff_python_ast::{ModModule, Stmt, name::Name, visitor::Visitor};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use serde::Serialize;
use smallvec::SmallVec;

mod builder;
mod forward;
mod module_scan;
mod names;

use builder::Builder;
pub(crate) use module_scan::{ModuleAssignment, module_assignments, module_bound_names};
pub(crate) use names::{
    ann_assign_with_named_field, bare_import_bound_name, from_import_bound_name, is_classvar,
    is_explicit_type_alias, is_screaming_case, sequence_elts, single_name_assignment,
    single_name_target, tail_identifier, top_level_module, type_head_identifier,
};

/// Module-wide binding-resolution table.
#[derive(Debug, PartialEq, Serialize)]
pub struct BindingAnalysis {
    #[serde(skip)]
    assignment_values: FxHashMap<TextSize, TextRange>,
    bindings: Vec<Binding>,
    #[serde(skip)]
    deleted: FxHashSet<Name>,
    #[serde(skip)]
    function_scope_at: FxHashMap<TextSize, ScopeId>,
    #[serde(skip)]
    global_writes: BTreeMap<Name, Vec<TextSize>>,
    #[serde(skip)]
    module_reads: OnceLock<Vec<(TextSize, Name)>>,
    scopes: Vec<Scope>,
    #[serde(skip)]
    unpack_targets: FxHashMap<BindingId, UnpackKind>,
}

impl BindingAnalysis {
    /// Walks `module` once and returns the resulting binding table.
    pub(crate) fn new(module: &ModModule) -> Self {
        let mut builder = Builder::new();
        builder.visit_body(&module.body);
        builder.finish()
    }

    fn binding(&self, id: BindingId) -> &Binding {
        &self.bindings[id.0 as usize]
    }

    fn module_binding(&self, name: &str) -> Option<&Binding> {
        self.scopes[0]
            .bindings
            .get(name)
            .map(|&id| self.binding(id))
    }

    /// Returns the number of write events recorded for `binding`.
    pub(crate) fn assignment_count(&self, binding: BindingId) -> usize {
        self.binding(binding).write_offsets.len()
    }

    /// Returns the source range of the value bound at `offset`, for a
    /// direct `name = value` or `name: T = value` write. `None` for a
    /// tuple/list target, a bare annotation, or an unrecorded offset.
    pub(crate) fn assignment_value_range(&self, offset: TextSize) -> Option<TextRange> {
        self.assignment_values.get(&offset).copied()
    }

    /// Returns the recorded write kinds for `binding`, in insertion
    /// order and without duplicates.
    pub(crate) fn binding_kinds(&self, binding: BindingId) -> &[BindingKind] {
        &self.binding(binding).kinds
    }

    /// Returns the source-text name of `binding`.
    pub(crate) fn binding_name(&self, binding: BindingId) -> &str {
        &self.binding(binding).name
    }

    /// Returns the binding ids declared directly inside the local
    /// scope of `stmt`. `stmt` must be a `Stmt::FunctionDef`. Any
    /// other statement yields an empty iterator.
    pub(crate) fn bindings_in_scope(
        &self,
        stmt: &Stmt,
    ) -> impl Iterator<Item = BindingId> + use<'_> {
        self.function_scope_at
            .get(&stmt.range().start())
            .copied()
            .into_iter()
            .flat_map(move |s| self.scopes[s.0 as usize].bindings.values().copied())
    }

    /// Returns `true` when any scope in the module binds `name`, the
    /// test that reports a builtin shadowed somewhere in the file.
    pub(crate) fn binds_name(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.bindings.contains_key(name))
    }

    /// Returns the offset of the earliest unconditional module-scope
    /// write of `name`, `None` when every write sits in a conditional
    /// branch (`if`/`for`/`while`/`try`/`match`) or `name` is unbound at
    /// module scope.
    pub(crate) fn first_unconditional_write(&self, name: &str) -> Option<TextSize> {
        self.module_binding(name)?.first_unconditional_write
    }

    /// Returns the offset of the earliest recorded write of `binding`.
    pub(crate) fn first_write_offset(&self, binding: BindingId) -> TextSize {
        self.binding(binding).write_offsets[0]
    }

    /// Returns `true` when `name` has a module-scope write at an offset
    /// strictly less than `offset`, a write nested in a conditional
    /// branch (`if`/`for`/`while`/`try`/`match`) included, whereas
    /// `is_defined_before` counts the unconditional writes alone.
    pub(crate) fn is_bound_before(&self, name: &str, offset: TextSize) -> bool {
        let module_write = self
            .module_binding(name)
            .is_some_and(|binding| binding.write_offsets[0] < offset);
        module_write
            || self
                .global_writes
                .get(name)
                .is_some_and(|offsets| offsets.iter().any(|&write| write < offset))
    }

    /// Returns `true` when `name` has an unconditional module-scope
    /// write at an offset strictly less than `offset`. A write nested in
    /// a conditional branch (`if`/`for`/`while`/`try`/`match`) is excluded.
    pub(crate) fn is_defined_before(&self, name: &str, offset: TextSize) -> bool {
        self.first_unconditional_write(name)
            .is_some_and(|first| first < offset)
    }

    /// Returns `true` when a `del` statement anywhere in the module names
    /// `name`.
    pub(crate) fn is_deleted(&self, name: &str) -> bool {
        self.deleted.contains(name)
    }

    /// Returns the number of distinct attributes read off the
    /// module-scope binding for `name` (`os.environ` and `os.getcwd`
    /// count as two), or `0` when `name` is unbound at module scope.
    pub(crate) fn module_attribute_count(&self, name: &str) -> usize {
        self.module_binding(name)
            .map_or(0, |binding| binding.attributes.len())
    }

    /// Returns the recorded write kinds of the module-scope binding for
    /// `name`, empty when `name` is unbound at module scope.
    pub(crate) fn module_binding_kinds(&self, name: &str) -> &[BindingKind] {
        self.module_binding(name)
            .map_or(&[], |binding| binding.kinds.as_slice())
    }

    /// Returns the read offsets of the module-scope binding for `name`
    /// when its sole write is one function definition. `None` when
    /// `name` is unbound at module scope, rebound, written by anything
    /// other than a single `def`, or potentially rebound by a
    /// module-scope `from x import *`.
    pub(crate) fn module_function_reads(&self, name: &str) -> Option<&[TextSize]> {
        // A `from x import *` binds under `*` rather than under each
        // real name it pulls in, so the visible `def` may not be the
        // function the call actually reaches, leaving no module name
        // safe to resolve against.
        if self.module_binding("*").is_some() {
            return None;
        }
        let binding = self.module_binding(name)?;
        (binding.kinds.as_slice() == [BindingKind::FunctionDef] && binding.write_offsets.len() == 1)
            .then_some(binding.read_offsets.as_slice())
    }

    /// The names the module scope binds that are read inside each of
    /// `ranges`, which ascend and never overlap, one set per range.
    pub(crate) fn module_names_read_within(&self, ranges: &[TextRange]) -> Vec<FxHashSet<&str>> {
        let mut read = vec![FxHashSet::default(); ranges.len()];
        let (Some(first), Some(last)) = (ranges.first(), ranges.last()) else {
            return read;
        };
        let reads = self.module_reads();
        let from = reads.partition_point(|&(offset, _)| offset < first.start());
        for (offset, name) in &reads[from..] {
            if *offset >= last.end() {
                break;
            }
            let slot = ranges.partition_point(|range| range.end() <= *offset);
            if ranges
                .get(slot)
                .is_some_and(|range| range.contains(*offset))
            {
                read[slot].insert(name.as_str());
            }
        }
        read
    }

    /// Every module-scope read as its offset beside the name read, in
    /// offset order. Built on first use, so a caller asking about one
    /// body's ranges walks the reads inside that span rather than every
    /// read the file holds.
    fn module_reads(&self) -> &[(TextSize, Name)] {
        self.module_reads.get_or_init(|| {
            let mut reads: Vec<(TextSize, Name)> = self.scopes[0]
                .bindings
                .iter()
                .flat_map(|(name, &id)| {
                    self.binding(id)
                        .read_offsets
                        .iter()
                        .map(move |&offset| (offset, name.clone()))
                })
                .collect();
            reads.sort_unstable_by_key(|&(offset, _)| offset);
            reads
        })
    }

    /// Returns `true` when the module reads the module-scope binding for
    /// `name` somewhere only data stands and nowhere a type stands. One
    /// read in an annotation outranks every data read, and a name the
    /// module never reads yields `false`.
    pub(crate) fn module_reads_as_data(&self, name: &str) -> bool {
        self.module_binding(name)
            .is_some_and(|binding| binding.runtime_read && !binding.annotation_read)
    }

    /// Returns `true` when the module-scope binding for `name` carries
    /// more than one write or an augmented-assignment write, and
    /// `false` when `name` is write-once or unbound at module scope.
    pub(crate) fn module_reassigned(&self, name: &str) -> bool {
        self.module_reassigned_without(name, |_| false)
    }

    /// Returns `true` when the module-scope binding for `name` carries
    /// more than one write `dropped` does not answer `true` for, or an
    /// augmented-assignment write.
    pub(crate) fn module_reassigned_without(
        &self,
        name: &str,
        dropped: impl Fn(TextSize) -> bool,
    ) -> bool {
        self.module_binding(name).is_some_and(|binding| {
            binding
                .write_offsets
                .iter()
                .filter(|&&offset| !dropped(offset))
                .nth(1)
                .is_some()
                || binding.kinds.contains(&BindingKind::AugAssign)
        })
    }

    /// Returns the number of reads recorded against the module-scope
    /// binding for `name`, `0` when `name` is unbound at module scope. A
    /// read a nested scope shadows counts against that scope's binding
    /// rather than this one.
    pub(crate) fn module_usage_count(&self, name: &str) -> usize {
        self.module_binding(name)
            .map_or(0, |binding| binding.read_offsets.len())
    }

    /// Returns `true` when the module-scope binding for `name` is read
    /// without an attribute access anywhere (the namespace object
    /// itself is used), and `false` when `name` is only attribute-read
    /// or unbound at module scope.
    pub(crate) fn module_used_bare(&self, name: &str) -> bool {
        self.module_binding(name)
            .is_some_and(|binding| binding.bare_read)
    }

    /// Returns every offset at which `binding` is read, ascending. A
    /// walrus target counts its own value as one of them.
    pub(crate) fn read_offsets(&self, binding: BindingId) -> &[TextSize] {
        &self.binding(binding).read_offsets
    }

    /// Returns `true` when the local scope of `stmt` binds `name`.
    /// `stmt` must be a `Stmt::FunctionDef`, and any other statement
    /// yields `false`.
    pub(crate) fn scope_binds(&self, stmt: &Stmt, name: &str) -> bool {
        self.bindings_in_scope(stmt)
            .any(|id| self.binding_name(id) == name)
    }

    /// Returns the unpack disposition of `binding` when its sole write
    /// is a multi-name tuple or list unpack target, `None` otherwise.
    pub(crate) fn unpack_target(&self, binding: BindingId) -> Option<UnpackKind> {
        self.unpack_targets.get(&binding).copied()
    }
}

/// Stable handle to a binding in `BindingAnalysis`. Cheap to copy.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct BindingId(u32);

/// Categories of write event recorded against a binding.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) enum BindingKind {
    Assignment,
    AugAssign,
    ClassDef,
    Comprehension,
    ExceptHandler,
    For,
    FunctionDef,
    Import,
    Parameter,
    Walrus,
    With,
}

/// Disposition of a multi-name unpack target for `inlinable-bindings`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum UnpackKind {
    /// Flagged with a subscript rewrite: the right-hand-side range and
    /// this target's index.
    Suggested(TextRange, usize),
    /// No subscript rewrite resolves, because a sibling target reads
    /// more than once, the right-hand side is a call, or a starred
    /// target shifts the indices.
    Unresolved,
}

/// One named binding in some scope, with every observed write and read.
/// `attributes` collects the distinct attribute names read off the
/// binding (`os.environ` records `environ`), `bare_read` flips when the
/// name is read without an attribute access (`foo(os)`), and
/// `first_unconditional_write` holds the earliest write not nested in a
/// conditional branch (`if`/`for`/`while`/`try`/`match`), or `None` when
/// every write is conditional.
#[derive(Debug, PartialEq, Serialize)]
struct Binding {
    annotation_read: bool,
    attributes: BTreeSet<Name>,
    bare_read: bool,
    first_unconditional_write: Option<TextSize>,
    kinds: SmallVec<[BindingKind; 2]>,
    name: Name,
    read_offsets: SmallVec<[TextSize; 4]>,
    runtime_read: bool,
    scope: ScopeId,
    write_offsets: SmallVec<[TextSize; 2]>,
}

/// One lexical scope plus its binding table keyed by name.
#[derive(Debug, PartialEq, Serialize)]
struct Scope {
    bindings: IndexMap<Name, BindingId, FxBuildHasher>,
    #[serde(skip)]
    globals: BTreeSet<Name>,
    kind: ScopeKind,
    parent: Option<ScopeId>,
}

/// Stable handle to a scope in `BindingAnalysis`. Cheap to copy.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
struct ScopeId(u32);

/// Categories of lexical scope.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
enum ScopeKind {
    Class,
    Comprehension,
    Function,
    Module,
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use indoc::indoc;
    use proptest::prelude::*;
    use rstest::rstest;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::testing::parse;

    fn analyze(src: &str) -> BindingAnalysis {
        BindingAnalysis::new(parse(src).ast())
    }

    fn module_binding_id(analysis: &BindingAnalysis, name: &str) -> BindingId {
        analysis.scopes[0]
            .bindings
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("no module-scope binding for {name:?}"))
    }

    #[test]
    fn bindings_in_scope_iterates_function_locals() {
        let source = parse("def f(a, b):\n    c = a + b\n    return c\n");
        let analysis = BindingAnalysis::new(source.ast());
        let stmt = &source.ast().body[0];
        let names: Vec<&str> = analysis
            .bindings_in_scope(stmt)
            .map(|id| analysis.bindings[id.0 as usize].name.as_str())
            .collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn bindings_in_scope_returns_empty_for_non_function_stmt() {
        let source = parse("x = 1\n");
        let analysis = BindingAnalysis::new(source.ast());
        let stmt = &source.ast().body[0];
        assert!(analysis.bindings_in_scope(stmt).next().is_none());
    }

    #[rstest]
    fn binds_name_rejects_a_name_the_module_never_binds(
        #[values("", "xs = [1]\n", "xs = list(rows)\n")] src: &str,
    ) {
        assert!(!analyze(src).binds_name("list"));
    }

    #[rstest]
    #[case::comprehension_target("xs = [set for set in rows]\n", "set")]
    #[case::function_local("def f():\n    dict = {}\n    return dict\n", "dict")]
    #[case::import("from x import list\n", "list")]
    #[case::module_assignment("list = [9]\n", "list")]
    #[case::parameter("def f(tuple):\n    return tuple\n", "tuple")]
    fn binds_name_reports_a_name_bound_in_any_scope(#[case] src: &str, #[case] name: &str) {
        assert!(analyze(src).binds_name(name));
    }

    #[test]
    fn deferred_read_resolves_to_an_enclosing_function_local() {
        let analysis = analyze(indoc! {"
            def outer():
                def inner():
                    return helper()
                def helper():
                    return 1
        "});
        let outer = analysis
            .scopes
            .iter()
            .find(|scope| scope.parent == Some(ScopeId(0)))
            .expect("outer is the function scope under module");
        let helper = *outer.bindings.get("helper").expect("helper bound in outer");
        assert_eq!(
            analysis.read_offsets(helper).len(),
            1,
            "the forward call resolves to outer's local",
        );
    }

    #[rstest]
    #[case::unconditional_write("Helper = int\n", "Helper", Some(0))]
    #[case::unconditional_after_conditional(
        "if flag:\n    Helper = str\nHelper = int\n",
        "Helper",
        Some(26)
    )]
    #[case::conditional_only_write("if flag:\n    Helper = int\n", "Helper", None)]
    #[case::undefined_name("x = 1\n", "y", None)]
    fn first_unconditional_write_reads_the_earliest_unconditional_offset(
        #[case] src: &str,
        #[case] name: &str,
        #[case] expected: Option<u32>,
    ) {
        assert_eq!(
            analyze(src).first_unconditional_write(name),
            expected.map(TextSize::new)
        );
    }

    #[rstest]
    #[case::conditional_only_write("if flag:\n    Helper = int\n", "Helper", 100)]
    #[case::except_only_write("try:\n    pass\nexcept E:\n    Helper = int\n", "Helper", 100)]
    #[case::for_only_write("for _ in xs:\n    Helper = int\n", "Helper", 100)]
    #[case::match_case_only_write("match x:\n    case 1:\n        Helper = int\n", "Helper", 100)]
    #[case::try_only_write("try:\n    Helper = int\nexcept E:\n    pass\n", "Helper", 100)]
    #[case::global_write_in_a_function(
        "def setup():\n    global Helper\n    Helper = int\n",
        "Helper",
        100
    )]
    #[case::unconditional_write("Helper = int\n", "Helper", 100)]
    #[case::while_only_write("while flag:\n    Helper = int\n", "Helper", 100)]
    fn is_bound_before_counts_a_write_inside_a_branch(
        #[case] src: &str,
        #[case] name: &str,
        #[case] offset: u32,
    ) {
        assert!(analyze(src).is_bound_before(name, TextSize::new(offset)));
    }

    #[rstest]
    #[case::function_local_write("def f():\n    Helper = int\n", "Helper", 100)]
    #[case::nonlocal_write_stays_local(
        "def outer():\n    Helper = 0\n    def inner():\n        nonlocal Helper\n        Helper = int\n",
        "Helper",
        200
    )]
    #[case::undefined_name("x = 1\n", "y", 100)]
    #[case::write_after_offset("x = 1\n", "x", 0)]
    fn is_bound_before_is_false_without_a_prior_module_write(
        #[case] src: &str,
        #[case] name: &str,
        #[case] offset: u32,
    ) {
        assert!(!analyze(src).is_bound_before(name, TextSize::new(offset)));
    }

    #[rstest]
    #[case::conditional_only_write("if flag:\n    Helper = int\n", "Helper", 100)]
    #[case::elif_only_write("if a:\n    pass\nelif b:\n    Helper = int\n", "Helper", 100)]
    #[case::except_only_write("try:\n    pass\nexcept E:\n    Helper = int\n", "Helper", 100)]
    #[case::for_only_write("for _ in xs:\n    Helper = int\n", "Helper", 100)]
    #[case::match_case_only_write("match x:\n    case 1:\n        Helper = int\n", "Helper", 100)]
    #[case::nested_conditional("if a:\n    if b:\n        Helper = int\n", "Helper", 100)]
    #[case::try_only_write("try:\n    Helper = int\nexcept E:\n    pass\n", "Helper", 100)]
    #[case::undefined_name("x = 1\n", "y", 100)]
    #[case::while_only_write("while flag:\n    Helper = int\n", "Helper", 100)]
    #[case::write_after_offset("x = 1\n", "x", 0)]
    fn is_defined_before_is_false_without_a_prior_unconditional_write(
        #[case] src: &str,
        #[case] name: &str,
        #[case] offset: u32,
    ) {
        assert!(!analyze(src).is_defined_before(name, TextSize::new(offset)));
    }

    #[rstest]
    #[case::unconditional_after_conditional(
        "if flag:\n    Helper = str\nHelper = int\n",
        "Helper",
        100
    )]
    #[case::unconditional_before_conditional(
        "Helper = str\nif flag:\n    Helper = int\n",
        "Helper",
        100
    )]
    #[case::finally_write_is_unconditional(
        "try:\n    pass\nfinally:\n    Helper = int\n",
        "Helper",
        100
    )]
    #[case::with_body_is_unconditional("with ctx() as _:\n    Helper = int\n", "Helper", 100)]
    #[case::prior_module_write("x = 1\nprint(x)\n", "x", 10)]
    fn is_defined_before_is_true_with_a_prior_unconditional_write(
        #[case] src: &str,
        #[case] name: &str,
        #[case] offset: u32,
    ) {
        assert!(analyze(src).is_defined_before(name, TextSize::new(offset)));
    }

    #[rstest]
    #[case::module_scope("import os\ndel os\n", "os", true)]
    #[case::function_scope("def f(x):\n    del x\n", "x", true)]
    #[case::attribute_target("import os\ndel os.path\n", "os", false)]
    #[case::subscript_target("d = {}\ndel d['k']\n", "d", false)]
    #[case::never_deleted("import os\nos.getcwd()\n", "os", false)]
    fn is_deleted_marks_only_a_bare_name_target(
        #[case] src: &str,
        #[case] name: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(analyze(src).is_deleted(name), expected);
    }

    #[test]
    fn module_attribute_count_counts_distinct_attributes() {
        let analysis = analyze("import os\nos.environ\nos.getcwd()\nos.environ\n");
        assert_eq!(analysis.module_attribute_count("os"), 2);
    }

    #[rstest]
    #[case("import os\n")]
    #[case("import os\nfoo(os)\n")]
    fn module_attribute_count_is_zero_without_attribute_reads(#[case] src: &str) {
        assert_eq!(analyze(src).module_attribute_count("os"), 0);
    }

    #[test]
    fn module_attribute_count_records_the_first_segment_of_a_chain() {
        let analysis = analyze("import os\nos.path.join('a', 'b')\n");
        assert_eq!(analysis.module_attribute_count("os"), 1);
    }

    #[test]
    fn module_function_reads_counts_each_in_module_reference() {
        let analysis = analyze("def f(b, a):\n    pass\n\n\nf(1, 2)\nf(3, 4)\n");
        let reads = analysis.module_function_reads("f").expect("unique def");
        assert_eq!(reads.len(), 2);
    }

    #[test]
    fn module_function_reads_excludes_a_shadowed_local_call() {
        let analysis = analyze("def f(b, a):\n    pass\n\n\ndef g(f):\n    f(1, 2)\n");
        assert!(
            analysis
                .module_function_reads("f")
                .expect("unique def")
                .is_empty(),
            "the call resolves to g's parameter, not module f",
        );
    }

    #[test]
    fn module_function_reads_includes_a_call_before_the_def() {
        let analysis = analyze("def caller():\n    return helper()\n\n\ndef helper():\n    pass\n");
        let reads = analysis
            .module_function_reads("helper")
            .expect("unique def");
        assert_eq!(
            reads.len(),
            1,
            "the forward reference resolves after the walk"
        );
    }

    #[test]
    fn module_function_reads_offset_points_at_the_call_callee() {
        let src = "def f(b, a):\n    pass\n\n\nf(1, 2)\n";
        let analysis = analyze(src);
        let reads = analysis.module_function_reads("f").expect("unique def");
        assert_eq!(reads.len(), 1);
        assert!(src[reads[0].to_usize()..].starts_with("f(1, 2)"));
    }

    #[test]
    fn module_function_reads_orders_a_forward_read_before_a_later_call() {
        let analysis = analyze(
            "def caller():\n    return helper()\n\n\ndef helper():\n    pass\n\n\nhelper()\n",
        );
        let reads = analysis
            .module_function_reads("helper")
            .expect("unique def");
        assert_eq!(reads.len(), 2);
        assert!(
            reads[0] < reads[1],
            "the deferred forward read sorts ahead of the later module-level call",
        );
    }

    #[rstest]
    #[case::star_after_def("def f(b, a):\n    pass\n\n\nfrom x import *\n")]
    #[case::star_before_def("from x import *\n\n\ndef f(b, a):\n    pass\n")]
    #[case::star_in_conditional_overlay(
        "try:\n    from x import *\nexcept ImportError:\n    pass\n\n\ndef f(b, a):\n    pass\n"
    )]
    fn module_function_reads_returns_none_under_a_module_star_import(#[case] src: &str) {
        assert!(analyze(src).module_function_reads("f").is_none());
    }

    #[rstest]
    #[case("def f():\n    pass\n\n\nf = 1\n")]
    #[case("f = lambda: 1\n")]
    #[case("x = 1\n")]
    fn module_function_reads_returns_none_unless_name_is_one_def(#[case] src: &str) {
        assert!(analyze(src).module_function_reads("f").is_none());
    }

    #[test]
    fn module_names_read_within_names_the_module_bindings_each_range_reaches() {
        let source = parse(indoc! {"
            def helper():
                return 1

            def outer(helper):
                return helper()

            def caller():
                return helper() + Outside
        "});
        let ranges: Vec<TextRange> = source.ast().body.iter().map(Ranged::range).collect();
        let read = source.binding_analysis().module_names_read_within(&ranges);
        assert!(
            read[1].is_empty(),
            "the parameter shadows the module helper"
        );
        assert_eq!(
            read[2],
            FxHashSet::from_iter(["helper"]),
            "an unbound name never enters"
        );
    }

    #[rstest]
    #[case("X = 1\n")]
    #[case("x = 1\n")]
    fn module_reassigned_is_false_for_write_once_or_unbound(#[case] src: &str) {
        assert!(!analyze(src).module_reassigned("X"));
    }

    #[rstest]
    #[case("X = 1\nX = 2\n")]
    #[case("X = 1\nX += 1\n")]
    #[case("X += 1\n")]
    fn module_reassigned_is_true_when_written_twice_or_augmented(#[case] src: &str) {
        assert!(analyze(src).module_reassigned("X"));
    }

    #[rstest]
    #[case("import os\nimport os\n")]
    #[case("import os\nimport os\nimport os\n")]
    fn module_reassigned_without_is_false_once_every_extra_write_drops(#[case] src: &str) {
        let analysis = analyze(src);
        let first = analysis.first_write_offset(module_binding_id(&analysis, "os"));

        assert!(analysis.module_reassigned("os"));
        assert!(!analysis.module_reassigned_without("os", |offset| offset != first));
    }

    #[test]
    fn module_reassigned_without_is_true_where_another_write_remains() {
        let analysis = analyze("import os\nimport os\nos = None\n");
        let first = analysis.first_write_offset(module_binding_id(&analysis, "os"));

        assert!(analysis.module_reassigned_without("os", |offset| offset == first));
    }

    #[test]
    fn module_used_bare_is_false_for_attribute_only_reads() {
        let analysis = analyze("import os\nos.getcwd()\nos.environ\n");
        assert!(!analysis.module_used_bare("os"));
    }

    #[rstest]
    #[case("import os\nfoo(os)\n")]
    #[case("import os\nx = os\n")]
    fn module_used_bare_is_true_for_a_namespace_reference(#[case] src: &str) {
        assert!(analyze(src).module_used_bare("os"));
    }

    #[test]
    fn type_alias_records_the_reads_in_its_bound() {
        let analysis = analyze(indoc! {"
            import collections.abc
            type Registry[T: collections.abc.Mapping] = dict[str, T]
        "});
        assert_eq!(
            analysis.module_attribute_count("collections"),
            1,
            "the PEP 695 bound reads `collections.abc`",
        );
    }

    #[rstest]
    #[case::reused_sibling(
        "head, tail = pair\nuse(tail)\nuse(tail)\nuse(head)\n",
        "head",
        Some(UnpackKind::Unresolved)
    )]
    #[case::call_value(
        "name, value = lookup()\nuse(name)\nuse(value)\n",
        "name",
        Some(UnpackKind::Unresolved)
    )]
    #[case::starred_target(
        "head, *rest = items\nuse(head)\nuse(rest)\n",
        "head",
        Some(UnpackKind::Unresolved)
    )]
    #[case::nested_unpack(
        "(a, b), c = pair\nuse(a)\nuse(b)\nuse(c)\n",
        "a",
        Some(UnpackKind::Unresolved)
    )]
    #[case::direct_assignment("x = 1\nuse(x)\n", "x", None)]
    #[case::single_name_unpack("(only,) = pair\nuse(only)\n", "only", None)]
    fn unpack_target_disposition(
        #[case] src: &str,
        #[case] name: &str,
        #[case] expected: Option<UnpackKind>,
    ) {
        let source = parse(src);
        let analysis = source.binding_analysis();
        assert_eq!(
            analysis.unpack_target(module_binding_id(analysis, name)),
            expected
        );
    }

    #[test]
    fn unpack_target_names_the_subscript_when_every_target_reads_once() {
        let source = parse("first, second = batch\nuse(first)\nuse(second)\n");
        let analysis = source.binding_analysis();
        let first = module_binding_id(analysis, "first");
        let second = module_binding_id(analysis, "second");
        assert_matches!(
            analysis.unpack_target(first),
            Some(UnpackKind::Suggested(range, 0)) if &source.text()[range] == "batch"
        );
        assert_matches!(
            analysis.unpack_target(second),
            Some(UnpackKind::Suggested(_, 1))
        );
    }

    #[test]
    fn unpack_target_subscript_handles_an_attribute_value() {
        let source = parse("x, y = box.pair\nuse(x)\nuse(y)\n");
        let analysis = source.binding_analysis();
        let x = module_binding_id(analysis, "x");
        assert_matches!(
            analysis.unpack_target(x),
            Some(UnpackKind::Suggested(range, 0)) if &source.text()[range] == "box.pair"
        );
    }

    #[rstest]
    #[case::if_test("if (n := f()):\n    pass\n", 1)]
    #[case::elif_test("if a:\n    pass\nelif (n := f()):\n    pass\n", 1)]
    #[case::while_test("while (n := f()):\n    pass\n", 1)]
    #[case::assignment_value("x = (n := f())\n", 1)]
    #[case::comprehension_guard("ys = [x for x in xs if (n := x)]\n", 1)]
    #[case::body_assignment("if a:\n    n = 1\n", 0)]
    #[case::if_body("if a:\n    print(n := f())\n", 1)]
    #[case::walrus_read_again("if (n := f()):\n    print(n)\n", 2)]
    fn walrus_target_counts_its_own_value_as_a_read(#[case] src: &str, #[case] reads: usize) {
        let analysis = analyze(src);
        assert_eq!(
            analysis
                .read_offsets(module_binding_id(&analysis, "n"))
                .len(),
            reads,
        );
    }

    proptest! {
        #[test]
        fn closure_binding_is_independent_of_outer_same_name(
            tail in "[a-z0-9]{0,5}"
        ) {
            let name = format!("x{tail}");
            let program = format!(
                "{name} = 1\ndef inner():\n    {name} = 2\n    return {name}\n",
            );
            let analysis = analyze(&program);
            let outer = module_binding_id(&analysis, &name);
            let inner_scope = analysis
                .scopes
                .iter()
                .find(|s| matches!(s.kind, ScopeKind::Function))
                .expect("inner is a function scope");
            let inner = *inner_scope
                .bindings
                .get(name.as_str())
                .expect("inner shadows name");
            prop_assert_ne!(outer, inner);
            prop_assert_eq!(analysis.read_offsets(outer).len(), 0);
            prop_assert_eq!(analysis.read_offsets(inner).len(), 1);
        }

        #[test]
        fn name_read_once_reports_one_read_offset(
            tail in "[a-z0-9]{0,5}"
        ) {
            let name = format!("x{tail}");
            let program = format!("{name} = 1\nprint({name})\n");
            let analysis = analyze(&program);
            let id = module_binding_id(&analysis, &name);
            prop_assert_eq!(analysis.read_offsets(id).len(), 1);
        }

        #[test]
        fn unread_name_reports_no_read_offset(
            tail in "[a-z0-9]{0,5}"
        ) {
            let name = format!("x{tail}");
            let program = format!("{name} = 1\n");
            let analysis = analyze(&program);
            let id = module_binding_id(&analysis, &name);
            prop_assert_eq!(analysis.read_offsets(id).len(), 0);
        }
    }
}
