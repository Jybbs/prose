# Usage

The Usage chapters walk the day-one path from a fresh install to a CI-gated `prose check`, with each page below stepping through one piece of that path. The section is operations rather than lookup, so the writing leans toward narrative against a working project rather than enumeration of every flag.

## Pick Your Run Shape

The invocation shapes below cover almost every run, with an additional shape for adopting one rule at a time. Each row pairs the use case on the left with the command pulled into the right margin.

<div class="pq-rows">

<div class="pq-row">

**Gate CI on pending rewrites.** The [**Quick Start**](/usage/quick-start) walks the day-one shape end to end, and the [**Exit Codes**](/reference/exit-codes) reference carries the gate contract every CI workflow compiles against.

<aside class="pq-aside"><code>prose check .</code></aside>

</div>

<div class="pq-row">

**Rewrite the working tree in place.** The most common operation against a project root, walked end to end in the [**Quick Start**](/usage/quick-start).

<aside class="pq-aside"><code>prose format .</code></aside>

</div>

<div class="pq-row">

**Preview a rewrite before it lands.** Prints a unified diff against the working tree without touching the files. Every flag and its precedence is documented in the [**CLI Reference**](/reference/cli).

<aside class="pq-aside"><code>prose format --diff .</code></aside>

</div>

<div class="pq-row">

**Read source from stdin and write to stdout.** The shape an editor save reaches for, with the [**Editor**](/integrations/editor) integration covering the LSP and save-hook paths.

<aside class="pq-aside"><code>prose format -</code></aside>

</div>

<div class="pq-row">

**Adopt one rule at a time.** Restricts the active set to a single slug for incremental rollout. The [**Quick Start**](/usage/quick-start) walks the path under *Subset the active rules*.

<aside class="pq-aside"><code>prose check --select &lt;slug&gt; .</code></aside>

</div>

</div>

## The Section at a Glance

- [**Installation**](/usage/installation) covers the package managers, post-install verification, and the platform matrix.
- [**Quick Start**](/usage/quick-start) walks each run shape end to end against a sample project.
- [**Suppression**](/usage/suppression) covers `# fmt: off`, `# fmt: skip`, and `# prose: ignore[<rule>]`, with one opt-out surface per scope.

## See Also

For the integration surfaces that hook `prose format` / `prose check` into the editor save event, the git staging boundary, and the CI gate, see [**Integrations**](/integrations/). For the rule catalog *Prose* actually runs, see [**Rules**](/rules/). For the primitive surface a downstream Rust caller links against, see [**Primitives**](/primitives/).
