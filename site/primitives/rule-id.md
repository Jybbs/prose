---
stability: public
---

# RuleId

<PrimitiveLayout primitive="rule-id">

Every rule needs a stable identifier the rest of the system can route off. The CLI's `--select` and `--ignore` flags parse names, `pyproject.toml`'s `[tool.prose.rules.<name>]` sub-tables key into names, suppression directives reference names inside `# prose: ignore[<name>]`, and diagnostic output routes by name. *RuleId* is the single canonical handle: a newtype wrapping a kebab-case slug *(`"align-equals"`, `"single-use-variables"`)* with equality, hashing, parsing, and the registry lookup that the [[pipeline]] drives off.

## Public Surface

`RuleId` is fully public in `0.2.x`, so a downstream Rust consumer constructs *RuleId* values, parses them from CLI or config input, prints them, and uses them as `HashMap` keys without restriction.

### Construction

- `From<&'static str> for RuleId` constructs a *RuleId* from a static slug. Used by the registry macro that emits each rule's slug constant at compile time.
- `FromStr for RuleId` parses runtime strings (CLI flags, config keys, suppression directives) into *RuleId* values, returning `ParseRuleIdError(pub String)` when the input is not a registered slug. The unknown slug lands in the tuple struct's `.0` field.

Snake-case input *(`align_equals`)* is normalized to the canonical kebab form before lookup, so a consumer accepting either shape gets the same result.

### Readers

- `as_str(&self) -> &'static str` returns the underlying slug, useful in diagnostic emission and config error messages.
- `Display` and `Debug` impls write the slug directly, so `format!("{id}")` and `{id:?}` produce `align-equals` rather than a wrapper-shaped debug representation.

### Equality and Hashing

*RuleId* derives `Clone, Copy, Eq, Hash, PartialEq`, so a downstream can use it as a `HashMap` key without ceremony. `Ord` / `PartialOrd` are not derived in `0.2.x`, so a consumer that needs sorted output sorts a slice by `RuleId::as_str` instead. *RuleId* is `Send + Sync` *(it wraps `&'static str`)*, which makes it cheap to ship across thread boundaries.

## Registry Pattern

Each concrete rule lives under `prose::rules` (*a `pub(crate)` module today*). The registry macro in `src/rule.rs` emits a single source of truth that drives every consumer:

- A `KNOWN_IDS: &[RuleId]` constant carrying every registered slug in canonical order.
- The pipeline constructors (`for_rule`, `with_defaults`, `with_filters`) that dispatch on slug.
- The slug-validity and uniqueness assertions, checked at compile time, so adding a malformed slug fails the build.
- The per-rule message strings consumed by diagnostic emission.

`Pipeline::known_ids() -> &'static [RuleId]` is the public entry point that exposes the canonical-order list to downstream consumers.

## Re-Using This Primitive

A downstream Rust consumer that builds a custom pipeline imports *RuleId*, parses user input into the type, and hands the resulting slices to `Pipeline::with_filters`:

```rust
use prose::pipeline::Pipeline;
use prose::rule::RuleId;
use std::str::FromStr;

let select: Vec<RuleId> = ["align-equals", "align-colons"]
    .iter()
    .map(|s| RuleId::from_str(s))
    .collect::<Result<_, _>>()?;
let pipeline = Pipeline::with_filters(&config, &select, &[]);
```

The `From<&'static str>` path is reserved for compile-time slug literals *(the registry macro)*. Runtime parsing always goes through `FromStr`, which gates on the registered-slug list and so prevents typos from leaking past the parse boundary.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page.

<template #related>

- [[pipeline]] iterates rules by *RuleId* in the registry's pinned order, and exposes `known_ids()` for consumers that need the full list.
- [[source]] carries diagnostics that reference rules by *RuleId*, so structured output formats (JSON, SARIF, GitHub annotations) all route by slug.
- [[suppression-map]] parses *RuleId* values out of `# prose: ignore[<slug>]` directives.

For the CLI surface that takes *RuleId* lists, the [**Quick Start**](/usage/quick-start#subset-the-active-rules) chapter covers the `--select` / `--ignore` arguments. For the rule catalog itself, the [**Rules**](/rules/) page walks every registered slug by category.

</template>

</PrimitiveLayout>
