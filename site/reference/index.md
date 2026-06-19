# Reference

Reference is the lookup surface for everything *Prose* exposes once a project is past the [**Usage**](/usage/) chapters. Where Usage walks workflows and Integrations wraps them at boundaries, Reference answers shaped questions about flags, facets, codes, diagnostic shapes, suppression directives, and the deterministic order rules fire in.

The [**Glossary**](/reference/glossary) is the substrate underneath every other reference page, because every `[[term]]` link on every page across the whole site lands there. Start at the Glossary when a word in the docs needs a definition rather than a workflow.

## A–Z Token Index

Every CLI flag, configuration key, exit code, output format, subcommand, and suppression directive *Prose* exposes. Hover any entry for its description and the destination page that documents it in full.

<AzIndex />

## The Section at a Glance

- [**Cache**](/reference/cache) covers the user-level cache, the `[cache]` facets, the `--no-cache` flag, and the `prose cache clean` subcommand.
- [**CLI**](/reference/cli) covers every flag, its precedence, and the subcommand it belongs to.
- [**Configuration**](/reference/configuration) covers the `prose.toml` and `pyproject.toml` config files and per-rule facets.
- [**Exit Codes**](/reference/exit-codes) covers the five-code contract CI gates compile against.
- [**Output Formats**](/reference/output-formats) covers `text`, `json`, `github`, and `sarif` shapes.
- [**Pipeline Order**](/reference/pipeline-order) covers the deterministic order rules fire in, with rationale per rule.
- [**Suppression Directives**](/reference/suppression-directives) covers `# fmt: off / on`, `# fmt: skip`, `# yapf` aliases, and `# prose: ignore` / `# prose: keep`.
- [**Glossary**](/reference/glossary) covers every term used in the docs, with cross-links to every page that introduces it.

## See Also

For the workflow context behind any of these surfaces, see [**Usage**](/usage/) and [**Integrations**](/integrations/). For the rule catalog the references describe, see [**Rules**](/rules/). For the primitive surface a downstream Rust caller links against, see [**Primitives**](/primitives/).
