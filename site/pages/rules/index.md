---
description: "Every rule Prose runs, listed from A to Z beside its family."
---

Every rule belongs to one family, whose page sets out what its rules share, and the [**Rule Composition**](/rules/composition/) page runs several rules together over one source.

## The Section at a Glance

<SectionGlance section="rules" />

## Every Rule from A to Z

Pressing a family's chip in the key below gathers that family's rules above the letters until a second press, Escape, or All families returns them, and the arrow beside each chip opens the family's page. Each family's badge leads each of its slugs in the list from A to Z.

The [**suppression directives**](/reference/suppression-directives) silence any rule on a line or a block, and the [**`enabled`**](/reference/configuration#per-rule-facets) facet turns any rule off across a project without changing the rest of the pipeline. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each rule runs.

<RulesPlate />

## Subsetting

`--select` and `--ignore` restrict the active set for one run and take precedence over the set the configuration enables. The [**Precedence**](/reference/cli#precedence) section of the CLI reference covers how both flags combine.
