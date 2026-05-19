---
category : auto-fix
domain   : formatting
caption  : "`from __future__ import annotations` lines that no longer carry their weight on the target Python version"
related  : [legacy-union-syntax]
---

# unused-future-annotations

The `from __future__ import annotations` directive made forward-reference annotations possible on Python versions where the runtime evaluated annotations eagerly. PEP 749 lands deferred annotation evaluation in Python **3.14** by default whenever the file's annotations are typing-only, and the import becomes redundant. `unused-future-annotations` removes the import when removal is provably safe for the file.

Three branches actually fire the rewrite. The file may carry zero annotations (*the directive is unused outright*). The `target-version` may be 3.14 or higher (*the runtime defers annotation evaluation, so the directive carries no runtime weight*). Or every name appearing in every annotation may resolve to a module-scope binding before its first annotation use (*forward references aren't needed, so the runtime evaluates annotations eagerly without raising*). When none of those branches holds, the import stays in place.

::: tabs key:prose-target-version
== Python 3.10
The version-gated branch stays quiet. Removal fires only if the file has zero annotations or every annotation resolves to a module-scope binding before use.

== Python 3.11
Same as 3.10.

== Python 3.12
Same as 3.10.

== Python 3.13
Same as 3.10.

== Python 3.14
The version-gated branch fires. PEP 749 lands deferred annotation evaluation, so the directive is redundant for typing-only annotations and the import removes cleanly.
:::

## Configuration

<RuleConfigTable preset="toggle" />

The `target-version` field from the top-level [**Configuration**](/guide/configuration#top-level-keys) gates the rewrite per project.

## The Canonical Case

A file whose annotations are typing-only loses the `__future__` import when the target version allows safe removal.

<Fixture rule="unused_future_annotations" case="binding_safe_target_py313" />

## More Examples

<Fixture rule="unused_future_annotations" case="among_others_no_annotations" title="The Import Removes Cleanly From a Mixed Import Block" />

<Fixture rule="unused_future_annotations" case="among_others_annotations_last" title="The Import Removes Whether Other Imports Sit Before or After It" />

<Fixture rule="unused_future_annotations" case="annotation_in_for_body" title="A Runtime Consumer Inside the Body Keeps the Import" />

<Fixture rule="unused_future_annotations" case="async_def_annotation" title="Async Function Annotations Are Recognized" />

<Fixture rule="unused_future_annotations" case="class_scope_ann_assign" title="Class-Scope Annotated Assignments Are Recognized Too" />

<Fixture rule="unused_future_annotations" case="directive_with_asname" title="An Aliased Directive Is Recognized Through the `as` Form" />

<Fixture rule="unused_future_annotations" case="fmt_off_suppresses" title="A `# fmt: off` Block Suppresses the Rewrite" />

## Related

<RelatedRulesInline />

For the gate semantics, [**`target-version`**](/guide/configuration#top-level-keys) in the Configuration chapter covers how the field is read across version-gated rules.
