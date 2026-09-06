# Ordering Rules

The ordering rules reorder sibling AST nodes by a fixed key while keeping each node's attached comments with it and the gaps between sections as written. The shared machinery lives in the [[orderer]] primitive, and each rule supplies the classifier closure that names the sort key. A pin keeps specific items *(a class docstring, a module-level import above a divider comment)* in the slot the author wrote while the rest of the siblings reorder.

<RuleCardList family="ordering" />

The [[orderer]] primitive page covers how a comment attaches to a node and the `gap_override` machinery. The `# prose: keep` directive *(documented in [**Suppression Directives**](/reference/suppression-directives))* keeps a dict literal out of [[alphabetize-siblings]] when the entry order carries meaning no rule can read from the source, and the rule's `sort-dict-keys` facet turns the reorder off across a whole project.
