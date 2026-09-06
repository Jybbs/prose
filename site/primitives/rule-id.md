---
consumedBy: [pipeline, suppression-map]
consumes: []
layer: base
stability: public
summary: "The kebab-case slug that names each rule across the CLI, config, suppressions, and diagnostics."
tagline: canonical rule slug
---

# RuleId

<PrimitiveLayout primitive="rule-id">

*RuleId* is the stable identifier the rest of the system looks a rule up by, a newtype wrapping a kebab-case slug *(`"align-equals"`, `"inlinable-bindings"`)* with equality, hashing, parsing, and the registry lookup the [[pipeline]] reads. The CLI's `--select` and `--ignore` flags parse names, the `[rules]` table is keyed by name, a suppression directive names a rule inside `# prose: ignore[<name>]`, and diagnostic output reports by name, so one canonical handle serves every surface.

## Public Surface

`RuleId` is fully public today, so a downstream Rust consumer constructs *RuleId* values, parses them from CLI or config input, prints them, and uses them as `HashMap` keys without restriction.

### Construction

- `From<&'static str> for RuleId` constructs a *RuleId* from a static slug and is compiled only under `#[cfg(test)]`, so it serves the test suite rather than the registry.
- `FromStr for RuleId` parses a runtime string (a CLI flag, a config key, a suppression directive) into a *RuleId* value, returning `ParseRuleIdError` when the input is not a registered slug. The error keeps the unknown slug in a private field and prints it through `Display` as ``unknown rule id `<slug>` ``.

Lookup compares the input's bytes against each registered slug exactly, so snake-case input *(`align_equals`)* is not a registered slug and fails to parse.

### Readers

- `as_str(&self) -> &'static str` returns the underlying slug, for diagnostic emission and config error messages.
- The `Display` and `Debug` impls write the slug directly, so `format!("{id}")` and `{id:?}` both produce `align-equals` rather than a wrapper-style debug representation.

### Equality and Hashing

*RuleId* derives `Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd`, so a downstream uses it as a `HashMap` key and sorts a slice of them without ceremony. *RuleId* is `Send + Sync` *(it wraps `&'static str`)*, which makes it cheap to send across thread boundaries.

## Registry Pattern

Each concrete rule lives under `prose::rules`, a `pub` module. The registry macro in `crate/src/rules/registry.rs` emits a single source of truth that every consumer reads:

- A `KNOWN_IDS: &[RuleId]` constant carrying every registered slug in canonical order.
- The pipeline constructors (`for_rule`, `with_defaults`, `with_filters`) that dispatch on slug.
- The slug-validity and uniqueness assertions, checked at compile time, so adding a malformed slug fails the build.
- The per-rule dependency column naming the slugs each rule runs behind, asserted at compile time to name only rules placed earlier and read back through `prose rules --output-format json`.
- The per-rule message strings diagnostic emission reads.

`Pipeline::known_ids() -> &'static [RuleId]` is the public entry point that exposes the canonical-order list to downstream consumers.

## Re-Using This Primitive

A downstream Rust consumer that builds a custom pipeline imports *RuleId*, parses user input into the type, and hands the resulting slices to `Pipeline::with_filters`:

```rust
use prose::pipeline::Pipeline;
use prose::rules::RuleId;
use std::str::FromStr;

let select: Vec<RuleId> = ["align-equals", "align-colons"]
    .iter()
    .map(|s| RuleId::from_str(s))
    .collect::<Result<_, _>>()?;
let pipeline = Pipeline::with_filters(&config, &select, &[]);
```

The `From<&'static str>` path exists only under `#[cfg(test)]`, so every runtime parse goes through `FromStr`, which checks the registered-slug list and stops a typo at the parse boundary.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page.

<template #related>

- [[pipeline]] iterates rules by *RuleId* in the registry's pinned order, and exposes `known_ids()` for a consumer that needs the full list.
- [[source]] carries diagnostics that reference rules by *RuleId*, so the structured output formats (JSON, SARIF, GitHub annotations) all report by slug.
- [[suppression-map]] parses *RuleId* values out of `# prose: ignore[<slug>]` directives.

For the CLI flags that take *RuleId* lists, the [**Quick Start**](/usage/quick-start#subset-the-active-rules) chapter covers the `--select` / `--ignore` arguments. For the rule catalog itself, the [**Rules**](/rules/) page lists every registered slug by category.

</template>

</PrimitiveLayout>
