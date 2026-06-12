---
stability: public
---

# Pipeline

<PrimitiveLayout primitive="pipeline">

*Pipeline* is the value `prose format` and `prose check` resolve into. It carries the registered rules in their canonical order and exposes two ways to run them. `run` applies each rule's edits to a fresh buffer, reparses between rules so every downstream pass reads a settled AST, and emits the final [[source]] plus a diagnostic list, while `diagnose` collects every rule's findings against the source as written for reporting.

## Public Surface

`Pipeline` is fully public in `0.2.x`, so a downstream Rust consumer constructs one through the entry points below, runs it against a [[source]], and reads the returned text plus diagnostics. `Pipeline` is `Send + Sync`, which means a single instance can be shared across `rayon` workers via `Arc` and the same instance can drive many `run` calls in sequence, because `run` takes `&self` and consumes only the `Source` passed in.

### Constructors

1. `Pipeline::empty() -> Self` returns a pipeline with no rules registered, for tests or callers building a custom rule set.
2. `Pipeline::with_defaults(config: &Config) -> Self` builds the canonical pipeline from every rule whose `enabled` flag is set in `[tool.prose]`. The `prose server` formatting and diagnostics paths reach for this, and the CLI reaches the same set through `with_filters` with no flags.
3. `Pipeline::with_filters(config: &Config, select: &[RuleId], ignore: &[RuleId]) -> Self` applies the CLI's `--select` and `--ignore` semantics. A non-empty `select` replaces the configured-enabled set, an empty `select` falls back to it, and `ignore` subtracts from the base to yield `select - ignore`.
4. `Pipeline::for_rule(name: &str, config: &Config) -> Option<Self>` builds a single-rule pipeline for diagnostic isolation and `prose check --select <rule>` exact-rule paths. Returns `None` for an unrecognized slug.

### Enumeration

`Pipeline::known_ids() -> &'static [RuleId]` exposes the full registered-rule list in canonical order, with the same shape the CLI's `--help` consumes. Consumers driving custom UIs over the catalog read from this.

### Execution

`run(&self, source: Source) -> Result<(Source, Vec<Diagnostic>), PipelineError>` walks the registered rules in their canonical order. Each rule applies its edits, the pipeline reparses, and the new *Source* feeds the next rule, with the final text and every emitted diagnostic returned to the caller. Suppression is applied transparently inside `run`, with every `# fmt: off` block, `# fmt: skip` marker, and `# prose: ignore[<rule>]` directive consulted at the edit-emission boundary so suppressed edits and lint diagnostics never reach the returned vector.

`diagnose(&self, source: &Source) -> Vec<Diagnostic>` collects every enabled rule's findings against the unmodified source, applying no edits and never reparsing, so each range stays anchored to the source as written rather than to an intermediate rewrite. `prose check` and `prose server` report through `diagnose`, where a rendered diagnostic points at the file the author wrote, while `run` feeds the rewritten text behind `prose format`'s diff, on-disk rewrite, and would-reformat summary. Both consult the same [[suppression-map]] and rule set, diverging only in that `diagnose` reads every rule against the original where `run` reads each against the prior rule's reparsed output.

`Diagnostic` carries the per-finding payload returned in the `Vec`:

```rust
pub struct Diagnostic {
    pub fix      : Option<Vec<Edit>>, // the fix's edits, or `None` for lint-only findings
    pub message  : String,            // human-readable explanation of the finding
    pub range    : TextRange,         // source span the finding points at
    pub rule     : RuleId,            // slug of the rule that emitted the finding
    pub severity : Severity,          // `Format` for auto-fix, `Lint` for report-only
}
```

`Severity::Format` carries a `Some(fix)` payload the pipeline applies, whereas `Severity::Lint` carries `fix: None` and reports a finding the user has to resolve themselves. Consumers building structured output formats *(JSON, SARIF, GitHub annotations)* route by `rule` to associate findings with the originating slug.

`PipelineError` is `pub` and carries one variant:

```rust
pub enum PipelineError {
    Reparse { rule: RuleId, source: ParseError },
}
```

The variant captures the rule whose output failed to reparse plus the underlying `ParseError`. A `Reparse` error means a rule produced syntactically invalid Python, which is a rule-authoring bug, not a consumer-recoverable condition. The intermediate `Source` is dropped, leaving no partial output for the caller to inspect.

## Determinism

Rule order is fixed and the same every run, so a given source plus configuration always produces the same output. The registry pins the order explicitly through a single `register_rules!` macro invocation in `src/rule.rs`, and the pipeline runs rules in that order without parallelism inside one *Source*. Cross-source parallelism *(two files at once)* is the path-mode CLI's job, owned by the walker above the pipeline rather than inside it.

## Internal Surface

`Pipeline::from_rules` is `pub(crate)`, so a downstream cannot register a hand-rolled rule list in `0.2.x`. The `Rule` trait that concrete rules implement is also `pub(crate)`. Both surfaces stabilize toward `1.0`, where consumers will be able to compose custom rule sets and implement project-specific rules against a stable trait.

## Re-Using This Primitive

The canonical shape for a downstream Rust consumer is:

```rust
use prose::config::Config;
use prose::pipeline::Pipeline;
use prose::source::Source;

let config   = Config::default();
let pipeline = Pipeline::with_defaults(&config);
let source   = Source::from_path("example.py")?;
let (formatted, diagnostics) = pipeline.run(source)?;
println!("{}", formatted.text());
```

For a single-rule isolation, `Pipeline::for_rule("align-equals", &config)` returns a pipeline that runs only `align-equals` against the source.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page. The Python wheel exposes the CLI rather than the library, so a Python consumer reaches the same pipeline indirectly through the binary.

<template #related>

- [[source]] is the value the pipeline reads and re-emits, reparsed between rules so each downstream pass reads a settled AST.
- [[rule-id]] is the handle each rule registers under, consumed by the pipeline's deterministic ordering and surfaced through `known_ids`.
- [[suppression-map]] filters the pipeline's emitted edits and lint diagnostics, dropping suppressed entries before they surface to the caller.
- [[binding-analysis]] builds once per *Source* and feeds rules whose questions are binding-shaped.

For the rule catalog the pipeline iterates, the [**Rules**](/rules/) page walks every shipped rule by category, and the [**Pipeline Order**](/reference/pipeline-order) reference renders the canonical run order with the rationale per rule.

</template>

</PrimitiveLayout>
