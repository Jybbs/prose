---
description: "Every rule Prose runs, listed from A to Z beside its family."
---

Each rule belongs to one family, whose chip below opens the family's page and whose badge leads each of its slugs in the list from A to Z.

The [**suppression directives**](/reference/suppression-directives) silence any rule on a line or a block, and the [**`enabled`**](/reference/configuration#per-rule-facets) facet turns any rule off across a project without changing the rest of the pipeline. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each rule runs.

<RulesPlate />

## Subsetting

`--select` and `--ignore` restrict the active set for one run and take precedence over the set the configuration enables. The [**Precedence**](/reference/cli#precedence) section of the CLI reference covers how both flags combine.
