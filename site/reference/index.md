# Reference

Reference is where to look up any single thing *Prose* exposes once a project is past the [**Usage**](/usage/) chapters. Usage walks through workflows and Integrations wires them into an editor, a hook, or CI, whereas Reference answers a specific question about a flag, a facet, an exit code, a diagnostic format, a suppression directive, or the order rules run in.

The [**Glossary**](/reference/glossary) sits beneath every other reference page, because every `[[term]]` link across the site opens there. Start at the Glossary when a word in the docs needs a definition rather than a walkthrough.

## A–Z Token Index

The index below lists every CLI flag, configuration key, exit code, output format, subcommand, and suppression directive *Prose* exposes. Hover an entry for its description and the page that documents it in full.

<AzIndex />

## The Section at a Glance

- [**Cache**](/reference/cache) covers the per-user cache, the `[cache]` keys, the `--no-cache` flag, and the `prose cache` subcommands.
- [**CLI**](/reference/cli) covers every flag, how the flags combine, and the subcommand each belongs to.
- [**Configuration**](/reference/configuration) covers the `prose.toml`, `.config/prose.toml`, and `pyproject.toml` config files and every per-rule facet.
- [**Exit Codes**](/reference/exit-codes) covers what each exit code means, which is the contract a CI gate reads.
- [**Output Formats**](/reference/output-formats) covers the `text`, `json`, `github`, and `sarif` formats.
- [**Pipeline Order**](/reference/pipeline-order) covers the fixed order rules run in and why each rule sits where it does.
- [**Suppression Directives**](/reference/suppression-directives) covers `# fmt: off / on`, `# fmt: skip`, the `# yapf` aliases, `# prose: ignore`, and `# prose: keep`.
- [**Glossary**](/reference/glossary) covers every term the docs use, each linked to the page that introduces it.

## See Also

The [**Usage**](/usage/) and [**Integrations**](/integrations/) sections cover the workflows these pages support. [**Rules**](/rules/) lists the rules the references describe, and [**Primitives**](/primitives/) covers the Rust types a downstream crate links against.
