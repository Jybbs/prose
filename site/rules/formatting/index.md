# Formatting Rules

The formatting rules shape the surrounding scaffolding around statements *(blank-line counts between definitions, collection layout against a line budget, trailing-comma discipline, singleton-group strip)*. Each rule resolves a layout question that sits adjacent to alignment and ordering, with the rewrites typically narrower than the alignment rules and more pervasive than the ordering rules. The [[collection-layout]] rule reads against the `code-line-length` budget, with its `max-atomics-per-line` facet carrying the per-line cap for atomic-literal collections.

<RuleCardList family="formatting" />

For the per-rule facets, see the [**Configuration**](/reference/configuration) reference. For the pipeline order these rules fire in *(formatting rules run early so the layout surface is settled by the time alignment rules measure widths)*, see the [**Pipeline Order**](/reference/pipeline-order) reference.
