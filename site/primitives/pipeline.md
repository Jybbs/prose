---
consumedBy: [cli, wasm]
consumes: [edit, rule-id, source, suppression-map]
layer: orchestration
stability: public
summary: "Runs registered rules in deterministic order, splices each batch of independent rules in one pass and reparses between batches, returns the final source, and answers which rules a buffer still leaves unsettled."
tagline: deterministic rule runner
---

# Pipeline

<PrimitiveLayout primitive="pipeline">

*Pipeline* is the value `prose format` and `prose check` resolve into. It carries the registered rules in their canonical order and exposes them through a handful of readings. `run` splices the edits of each batch of consecutive rules the registry declares independent into a fresh buffer in one pass, reparses between batches so every downstream pass reads a settled AST, and emits the final [[source]] plus a diagnostic list, `diagnose` collects every rule's findings against the source as written for reporting, `settle_report` reads what a buffer a run has already produced still leaves behind, and `unsettled` answers the first part of that reading alone.

## Public Surface

`Pipeline` is fully public today, so a downstream Rust consumer constructs one through the entry points below, runs it against a [[source]], and reads the returned text plus diagnostics. `Pipeline` is `Send + Sync`, which means a single instance can be shared across `rayon` workers via `Arc` and the same instance can drive many `run` calls in sequence, because `run` takes `&self` and consumes only the `Source` passed in.

### Constructors

1. `Pipeline::empty() -> Self` returns a pipeline with no rules registered, for tests or callers building a custom rule set.
2. `Pipeline::with_defaults(config: &Config) -> Self` builds the canonical pipeline from every rule whose `enabled` flag is set in `[tool.prose]`. The `prose server` formatting and diagnostics paths and the WebAssembly bindings reach for this, and the CLI reaches the same set through `with_filters` with no flags.
3. `Pipeline::with_filters(config: &Config, select: &[RuleId], ignore: &[RuleId]) -> Self` applies the CLI's `--select` and `--ignore` semantics. A non-empty `select` replaces the configured-enabled set, an empty `select` falls back to it, and `ignore` subtracts from the base to yield `select - ignore`.
4. `Pipeline::for_rule(name: &str, config: &Config) -> Option<Self>` builds a single-rule pipeline for diagnostic isolation and `prose check --select <rule>` exact-rule paths. Returns `None` for an unrecognized slug.
5. `Pipeline::sharing(self, sharing: Sharing) -> Self` sets which rules a run lets share a splice and a parse. `Sharing::Declared`, the default, follows the registry's independence table, `Sharing::Never` reparses after every editing rule, the fold the subset probe measures a batched pair against, and `Sharing::Always` batches every rule and surfaces a batch the reparse rejects as `PipelineError::Batch`, the reading the probe takes to test whether a pair's edits are independent.

### Enumeration

`Pipeline::known_ids() -> &'static [RuleId]` exposes the full registered-rule list in canonical order, with the same shape the CLI's `--help` consumes. Consumers driving custom UIs over the catalog read from this.

### Splitting

`split(self) -> Vec<(RuleId, Self)>` breaks a pipeline into one single-rule pipeline per rule it carries, in order, each holding its rule exactly as the parent's selection constructed it. Several rules read a sibling's flag off the resolved selection, `band-constants` and `alphabetize-siblings` reading whether `group-imports` is enabled among them, so a rule built alone through `for_rule` is not the rule that runs beside its sibling, and `split` is how a consumer runs the two one at a time without changing what either read.

`fingerprint(&self) -> String` renders every carried rule's settings, equal for two pipelines whose rules resolved alike, so a consumer holding many single-rule pipelines can share the ones that would behave the same.

`fingerprints(&self) -> Vec<String>` renders one fingerprint per carried rule in registration order, each equal to what that rule's own single-rule pipeline renders, so a consumer comparing two selections seat by seat reads them without splitting either.

### Execution

`run(&self, source: Source) -> Result<(Source, Vec<Diagnostic>), PipelineError>` walks the registered rules in their canonical order. Each batch of consecutive rules the registry declares independent computes its edits against one buffer and weaves them in a single pass, the pipeline then rebuilding the tree through [[source]], which reparses only the statements those edits reached wherever it can and the whole file wherever it cannot, and the new *Source* feeds the next batch carrying whichever of the previous buffer's tables every member's edits left standing, with a rule seated behind one the batch holds, or whose edits overlap one already batched, opening the next batch against the reparsed buffer. A batch of several rules whose splice the reparse rejects replays them one at a time, so the error names the rule whose own edits fail. The final text and every emitted diagnostic return to the caller. Suppression is applied transparently inside `run`, with every `# fmt: off` block, `# fmt: skip` marker, and `# prose: ignore[<rule>]` directive consulted at the edit-emission boundary so suppressed fix groups and lint diagnostics never reach the returned vector. A fix group drops whole as soon as one of its edits falls under a directive, leaving a rule's co-dependent edits either all applied or all withheld.

`format(&self, source: Source) -> Result<Source, PipelineError>` makes the same fold and returns the settled text alone, skipping the diagnostics `run` collects and the lint pass it closes on, which is the entry a consumer wanting only the rewrite takes. `format_span(&self, source: Source, seats: Range<usize>) -> Result<Source, PipelineError>` bounds that fold to the rules seated in `seats`, so a caller holding the text a prefix of the fold produced resumes behind it rather than re-deriving it, with the compile gate reading the segment's entry source. The corpus sweep folds its width-and-axis slices as a tree through these two, each budget-narrowed slice resuming behind the seats it shares with an earlier slice.

`diagnose(&self, source: &Source) -> Vec<Diagnostic>` collects every enabled rule's findings against the unmodified source, applying no edits and never reparsing, so each range stays anchored to the source as written rather than to an intermediate rewrite. `prose check`, `prose server`, and a structured `format` report through `diagnose`, where a rendered diagnostic points at the file the author wrote, while `run` feeds the rewritten text behind `prose format`'s diff, on-disk rewrite, and would-reformat summary. Both consult the same [[suppression-map]] and rule set, diverging only in that `diagnose` reads every rule against the original where `run` reads each against the buffer its batch opened on.

`unsettled(&self, source: &Source) -> Vec<RuleId>` names every rule this pipeline carries whose edits would still rewrite `source`, and answers empty for a buffer that has settled. It reads the subset the pipeline was built with rather than the default set, so a `--select` run answers for that selection alone, and a file carrying a file-level `# prose: off` answers empty because no rule reaches it. Every `prose format` run makes this walk over each file it rewrote, narrowed through the crate-private `unsettled_among` to the rules that edited on the first pass, raising the [**unstable-output notice**](/reference/cli#unstable-output) where it names rules. `prose check --validate` and the corpus sweeps keep the full walk, and the sweep over each rule alone and each ordered rule pair is where a rule leaning on a later rule to finish its work surfaces.

`settle_report(&self, source: &Source) -> SettleReport` makes the same walk once and returns three things `unsettled` collapses into one, `editing` being the rules whose edits still rewrite `source` in registration order, `unlanded` the rules holding a fix group that splices back to the same text or does not apply, and `witness` the first editing rule paired with the text its edits weave, which a report shows as the rewrite. `unsettled` makes the same walk without weaving the witness and returns `editing` alone. The corpus sweep reads `settle_report` over every file a run produced, so a rule that is stable and incomplete at once surfaces beside a rule that keeps editing.

```rust
pub struct SettleReport {
    pub editing  : Vec<RuleId>,              // the rules whose edits still rewrite the buffer
    pub unlanded : Vec<RuleId>,              // the rules reporting a fix the weave never lands
    pub witness  : Option<(RuleId, String)>, // the first editing rule and the text it weaves
}
```

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

`PipelineError` is `pub` and carries a variant per failure the pipeline can surface:

```rust
pub enum PipelineError {
    Batch { rules: Vec<RuleId> },
    Cell { cell: OneIndexed, rule: RuleId, source: ParseError },
    Compile { error: SemanticSyntaxError, rule: RuleId },
    Reparse { rule: RuleId, source: ParseError },
}
```

Every variant names the rule whose output failed:

- A `Batch` error names every rule of a batch whose one-pass splice the reparse rejected, surfacing only under `Sharing::Always`, whereas the default sharing replays the batch's rules one at a time and reports the responsible rule through one of the other three.
- A `Cell` error means a notebook cell that parsed on its own before the rule ran no longer does, naming that cell by its position in the notebook.
- A `Compile` error means the output parses yet fails the semantic-syntax check Python's own `compile` applies.
- A `Reparse` error means a rule produced syntactically invalid Python.

The last three are rule-authoring bugs rather than consumer-recoverable conditions. The intermediate `Source` is dropped either way, leaving no partial output for the caller to inspect.

## Determinism

Rule order is fixed and the same every run, so a given source plus configuration always produces the same output. The registry pins the order explicitly through a single `register_rules!` macro invocation in `crate/src/rule/mod.rs`, and the pipeline runs rules in that order without parallelism inside one *Source*. Cross-source parallelism *(two files at once)* is the path-mode CLI's job, owned by the walker above the pipeline rather than inside it.

## Internal Surface

`Pipeline::from_rules` is `pub(crate)`, so a downstream cannot register a hand-rolled rule list today. The `Rule` trait that concrete rules implement is also `pub(crate)`, and each rule declares through it whether its edits leave every binding standing, which decides whether the next *Source* inherits the binding table across the reparse. Both surfaces stabilize toward `1.0`, where consumers will be able to compose custom rule sets and implement project-specific rules against a stable trait.

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

- [[source]] is the value the pipeline reads and re-emits, reparsed between batches of independent rules so each downstream pass reads a settled AST, with the binding table carried into the new value where every member of the batch keeps every binding.
- [[rule-id]] is the handle each rule registers under, consumed by the pipeline's deterministic ordering and surfaced through `known_ids`.
- [[suppression-map]] filters the pipeline's emitted edits and lint diagnostics, dropping suppressed entries before they surface to the caller.
- [[binding-analysis]] builds on first read, carries across a reparse where the rule between keeps every binding, and feeds rules whose questions are binding-shaped.

For the rule catalog the pipeline iterates, the [**Rules**](/rules/) page walks every shipped rule by category, and the [**Pipeline Order**](/reference/pipeline-order) reference renders the canonical run order with the rationale per rule.

</template>

</PrimitiveLayout>
