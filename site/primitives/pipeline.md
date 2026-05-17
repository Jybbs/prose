# Pipeline

*Pipeline* is the value `prose format` and `prose check` resolve into. It carries the registered rules in their canonical order, applies each rule's edits to a fresh buffer, reparses between rules so every downstream pass reads a settled AST, and emits the final [[source]] plus a diagnostic list at the end.

<DependencyGraph />

## Public API

`Pipeline` is fully public in `0.2.x`. A downstream Rust consumer constructs one through one of four entry points, runs it against a [[source]], and reads the returned text plus diagnostics.

**Constructors.**

- `Pipeline::empty() -> Self` returns a pipeline with no rules registered. Useful for tests or callers building a custom rule set.
- `Pipeline::with_defaults(config: &Config) -> Self` builds the canonical pipeline from every rule whose `enabled` flag is set in the project's `[tool.prose]` table. The `prose format` and `prose check` paths both reach for this.
- `Pipeline::with_filters(config: &Config, select: &[RuleId], ignore: &[RuleId]) -> Self` applies the CLI's `--select` and `--ignore` semantics. A non-empty `select` replaces the configured-enabled set, whereas an empty `select` falls back to it, after which `ignore` subtracts from the base to yield `select - ignore`.
- `Pipeline::for_rule(name: &str, config: &Config) -> Option<Self>` builds a single-rule pipeline, useful for diagnostic isolation and for `prose check --select <rule>` exact-rule paths. Returns `None` for an unrecognized slug.

**Enumeration.** `Pipeline::known_ids() -> &'static [RuleId]` exposes the full registered-rule list in canonical order, with the same shape the CLI's `--help` consumes. Consumers driving custom UIs over the catalog read from this.

**Execution.** `run(&self, source: Source) -> Result<(Source, Vec<Diagnostic>), PipelineError>` walks the registered rules in order, applies each rule's edits, reparses, hands the new *Source* to the next rule, and returns the final text plus every diagnostic emitted. `PipelineError` is `pub` and captures the parse-failure case when an intermediate reparse fails.

## Determinism

Rule order is deterministic. The registry pins it explicitly through a single `register_rules!` macro invocation in `src/rule.rs`, and the pipeline runs rules in that order without parallelism inside one *Source*. Cross-source parallelism (*two files at once*) is the path-mode CLI's job, owned by the walker above the pipeline rather than inside it.

## Reuse Pattern

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

## Internal Surface (`0.2.x`)

`Pipeline::from_rules` is `pub(crate)`, so a downstream cannot register a hand-rolled rule list in `0.2.x`. The `Rule` trait that concrete rules implement is also `pub(crate)`. Both surfaces stabilize toward `1.0`, where consumers will be able to compose custom rule sets and implement project-specific rules against a stable trait.

## Re-Using This Primitive

A downstream Rust crate consumes *prose* through a Git dependency pinned to a release tag:

```toml
[dependencies]
prose = { git = "https://github.com/Jybbs/prose", tag = "0.2.3" }
```

The Python wheel exposes the CLI rather than the library, so a Python consumer reaches the same pipeline indirectly through the binary.

## Related

- [[source]] is the value the pipeline reads and re-emits, reparsed between rules so each downstream pass reads a settled AST.
- [[rule-id]] is the handle each rule registers under, consumed by the pipeline's deterministic ordering and surfaced through `known_ids`.
- [[suppression-map]] filters the pipeline's emitted edits and lint diagnostics, dropping suppressed entries before they surface to the caller.
- [[binding-analysis]] builds once per *Source* and feeds rules whose questions are binding-shaped.

For the rule catalog the pipeline iterates, the [**Rules Overview**](/rules/) page walks every shipped rule by category.
