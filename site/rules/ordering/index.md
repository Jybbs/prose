---
description: "The rules that reorder sibling nodes by a fixed key while keeping each node's comments with it."
---

# Ordering Rules

The ordering rules reorder sibling AST nodes by a fixed key while keeping each node's attached comments with it and the gaps between sections as written. The shared machinery lives in the [[orderer]] primitive, and each rule supplies the classifier closure that names the sort key. A pin keeps specific items *(a class docstring, a module-level import above a divider comment)* in the slot the author wrote while the rest of the siblings reorder.

A **section marker** divides a run into sections that each reorder on their own while the marker keeps its place. The markers are:

- A banner whose rule of `=`, `-`, `*`, `_`, `#`, `~`, `─`, `━`, or `═` stands alone or opens or closes its label (*`# --- Lifecycle ---`, `# Lifecycle -------#`*)
- A `##` heading
- A suppression directive
- A tool pragma (*`# isort: split`, `# noqa`*)
- A comment set at a shallower indent than the members around it

A comment indented deeper than the member below it closes the member above and moves with that one, as a note on the last line of a function body or at the continuation indent of an `__all__` list does.

<RuleCardList family="ordering" />

The [[orderer]] primitive page covers how a comment attaches to a node and the `gap_override` machinery. The `# prose: keep` directive *(documented in [**Suppression Directives**](/reference/suppression-directives#construct-order-preservation))* keeps a dict literal, a dunder list, or a class body out of the ordering rules when the order written carries meaning no rule can read from the source, and the `sort-dict-keys` facet of [[alphabetize-siblings]] turns the dict reorder off across a whole project.
