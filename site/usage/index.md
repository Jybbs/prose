# Usage

The Usage chapters follow the path from a fresh install to a `prose check` step gating CI, one page per step. This section tells that story against a working project, whereas the [**Reference**](/reference/) section lists every flag and key.

## Pick Your Run Shape

The commands below cover almost every run, plus one for adopting a single rule at a time. Each row pairs what you want to do with the command that does it.

<div class="pq-rows">

<div class="pq-row">

**Gate CI on pending rewrites.** The command exits non-zero when any file would change, and the [**Quick Start**](/usage/quick-start) walks the full setup. The [**Exit Codes**](/reference/exit-codes) reference lists what each exit code means for a CI gate.

<aside class="pq-aside"><code>prose check .</code></aside>

</div>

<div class="pq-row">

**Rewrite a project in place.** The most common command, run against the project root and walked end to end in the [**Quick Start**](/usage/quick-start).

<aside class="pq-aside"><code>prose format .</code></aside>

</div>

<div class="pq-row">

**Preview a rewrite before writing it.** Prints a unified diff and leaves every file as it was. The [**CLI Reference**](/reference/cli) documents every flag and how the flags combine.

<aside class="pq-aside"><code>prose format --diff .</code></aside>

</div>

<div class="pq-row">

**Read source from stdin and write to stdout.** The form an editor's save hook uses, with the [**Editor**](/integrations/editor) page covering both the language server and the save-hook setup.

<aside class="pq-aside"><code>prose format -</code></aside>

</div>

<div class="pq-row">

**Adopt one rule at a time.** Runs a single rule, so a project can take the rules on one by one. The [**Quick Start**](/usage/quick-start) covers it under *Subset the active rules*.

<aside class="pq-aside"><code>prose check --select &lt;slug&gt; .</code></aside>

</div>

</div>

## The Section at a Glance

- [**Installation**](/usage/installation) covers the package managers, post-install verification, and the platforms wheels are built for.
- [**Quick Start**](/usage/quick-start) walks each command end to end against a sample project.
- [**Suppression**](/usage/suppression) covers `# fmt: off`, `# fmt: skip`, and `# prose: ignore[<rule>]`, one directive per scope.

## See Also

- [**Integrations**](/integrations/) covers running `prose format` or `prose check` from an editor's save event, a pre-commit hook, and a CI job.
- [**Rules**](/rules/) lists every rule *Prose* runs.
- [**Primitives**](/primitives/) covers the Rust types a downstream crate links against.
- [**Sandbox**](/sandbox/) runs the formatter in the browser on examples taken from the fixture set.
