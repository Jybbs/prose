import type { GlossaryFamily } from '../shared/registries'

export interface GlossaryEntry {
  aliases   ?: readonly string[]
  definition : string
  families  ?: readonly GlossaryFamily[]
  href      ?: string
  rule      ?: string
}

export const glossary: Record<string, GlossaryEntry> = {
  '# fmt: off': {
    aliases    : ['# fmt: on'],
    definition : '`# fmt: off` and `# fmt: on` mark a block, each on a comment line of its own. '
               + 'Every rule leaves the lines between them as written, rewrites and lints alike.',
    families   : ['formatting', 'engine'],
    href       : '/reference/suppression-directives#block-markers'
  },

  '# fmt: skip': {
    definition : '`# fmt: skip` at the end of a statement exempts that whole logical line from '
               + 'every rewriting rule, with no surrounding block markers needed.',
    families   : ['formatting', 'engine'],
    href       : '/reference/suppression-directives#rewrite-suppression'
  },

  '# prose: ignore': {
    aliases    : ['# prose: ignore[...]'],
    definition : '`# prose: ignore` at the end of a line silences lint diagnostics on it. The '
               + 'bracketed form names the rule slugs to silence, whereas the bare form '
               + 'silences every lint on that line.',
    families   : ['lint', 'engine'],
    href       : '/reference/suppression-directives#lint-suppression'
  },

  '--ignore': {
    definition : '`--ignore` removes the named rules from a single run. The flag repeats, and '
               + 'combined with `--select` it subtracts from the selected set.',
    families   : ['cli'],
    href       : '/usage/quick-start#subset-the-active-rules'
  },

  '--no-cache': {
    definition : '`--no-cache` bypasses the user-level cache for one `prose check` or '
               + '`prose format` run, overriding a configured `[cache] enabled = true`.',
    families   : ['cli'],
    href       : '/reference/cache#no-cache'
  },

  '--select': {
    definition : '`--select` restricts one run to the named rules. The flag repeats, and '
               + '`--ignore` then subtracts from that set.',
    families   : ['cli'],
    href       : '/usage/quick-start#subset-the-active-rules'
  },

  '--verbose': {
    definition : '`--verbose` prints a one-line count of cache hits and misses to stderr at '
               + 'the end of each `prose check` or `prose format` run, printing '
               + '`cache: bypassed` when the cache is off through `--no-cache` or '
               + '`[cache] enabled = false`.',
    families   : ['cli'],
    href       : '/reference/cache#hit-miss-telemetry'
  },

  'AST': {
    aliases    : ['abstract syntax tree'],
    definition : 'An AST is the tree `ruff_python_parser` builds from a file\'s text. *Prose* '
               + 'keeps it inside `Source` and rebuilds it between batches of rules, so each '
               + 'rule reads the tree as the rules before it left the text.',
    families   : ['engine'],
    href       : '/primitives/source'
  },

  'BindingAnalysis': {
    aliases    : ['binding analysis', 'binding map', 'name bindings', 'binding', 'bindings'],
    definition : '`BindingAnalysis` is a per-`Source` table recording every write and read of '
               + 'every name in every scope. It is built on first use and carried across a '
               + 'reparse whenever the rules in between changed no binding. '
               + '`inlinable-bindings` reads it.',
    families   : ['engine', 'lint'],
    href       : '/primitives/binding-analysis'
  },

  'BLAKE3': {
    definition : 'BLAKE3 is the hash function the cache keys on. The key digests the source '
               + 'bytes, the active configuration serialized as TOML, and the *Prose* '
               + 'version, so a change to any of the three produces a new key.',
    families   : ['engine', 'cli'],
    href       : '/reference/cache#key-shape'
  },

  'Diagnostic': {
    aliases    : ['diagnostic', 'diagnostics', 'lint diagnostic'],
    definition : 'A `Diagnostic` is the report a rule emits for one finding. It carries a '
               + 'severity, `Format` for a change `prose format` writes and `Lint` for a '
               + 'finding no rule rewrites.',
    families   : ['engine', 'lint']
  },

  'Pipeline': {
    aliases    : ['pipeline'],
    definition : 'The `Pipeline` runs every enabled rule over a `Source` in a fixed order, '
               + 'applies the edits of each batch of independent rules in one pass, rebuilds '
               + 'the tree between batches, and returns the final text with its diagnostics.',
    families   : ['engine'],
    href       : '/primitives/pipeline'
  },

  'Ruff': {
    aliases    : ['ruff'],
    definition : 'Ruff is Astral\'s Python linter and formatter. A project that runs Ruff '
               + 'alongside *Prose* turns off the `pycodestyle` whitespace checks that would '
               + 'report *Prose*\'s aligned columns, and the integration page lists them.',
    families   : ['engine'],
    href       : '/integrations/ruff'
  },

  'RuleId': {
    aliases    : ['rule id', 'rule IDs'],
    definition : 'A `RuleId` is a rule\'s slug, the kebab-case name that identifies it in CLI '
               + 'flags, config tables, suppression directives, and diagnostic output.',
    families   : ['engine', 'cli'],
    href       : '/primitives/rule-id'
  },

  'Source': {
    definition : '`Source` holds a parsed file, bundling its text, tree, token stream, line '
               + 'index, and suppression map. Every rule reads the file through this value.',
    families   : ['engine'],
    href       : '/primitives/source'
  },

  'SuppressionMap': {
    aliases    : [
      'suppression map', 'suppression directive', 'suppression directives', 'suppression'
    ],
    definition : '`SuppressionMap` is the per-`Source` index of every `# fmt: off`, '
               + '`# fmt: skip`, `# yapf`, and `# prose: ignore[...]` directive in the file. '
               + 'A rule checks it before emitting an edit or a diagnostic.',
    families   : ['engine', 'formatting', 'lint'],
    href       : '/primitives/suppression-map'
  },

  'alignment group': {
    aliases    : ['alignment groups', 'group', 'singleton group', 'singleton groups'],
    definition : 'An alignment group is a run of consecutive rows at the same indentation '
               + 'whose aligned token is padded to one column. A blank line, a comment line, '
               + 'or a statement of another kind ends the group, and each group aligns on '
               + 'its own.',
    families   : ['alignment'],
    href       : '/primitives/aligner'
  },

  'annotation': {
    aliases    : ['annotations', 'type annotation', 'type annotations'],
    definition : 'An annotation is the `name: Type` declaration on a parameter, a return, or '
               + 'a variable. `modernize-annotations` rewrites the older spellings, '
               + '`prune-inert-imports` counts the names an annotation reads, and '
               + '`alphabetize-siblings` treats an annotation evaluated at definition time '
               + 'as a reference that fixes the order of the definitions it names.',
    families   : ['formatting', 'alignment', 'ordering'],
    href       : 'https://docs.python.org/3/glossary.html#term-annotation'
  },

  'applicability': {
    definition : 'Applicability is the confidence level on an auto-fix\'s `fix` payload, in '
               + 'the shape Ruff\'s output uses. `safe` means the rewrite preserves runtime '
               + 'semantics and an editor can apply it without asking, whereas `unsafe` and '
               + '`display` are reserved in the schema for compatibility.',
    families   : ['cli', 'engine'],
    href       : '/reference/output-formats#json'
  },

  'atomic': {
    aliases    : ['atomic literal', 'atomic literals'],
    definition : 'An atomic is a single indivisible value, an integer, a float, a string, or '
               + 'a bare name. `reflow-collections` keeps a short collection of atomics on '
               + 'one line, since each entry reads at a glance.',
    rule       : 'reflow-collections'
  },

  'auto-fix': {
    aliases    : ['auto-fixes', 'auto-fixing', 'Auto-Fix'],
    definition : 'Auto-fix is the category of rules whose diagnostics carry a rewrite. '
               + '`prose format` writes the rewrite, and `prose check` reports it as a '
               + 'pending change.',
    families   : ['formatting']
  },

  'band': {
    aliases    : ['bands'],
    definition : 'The band is the block of module-level constants, type aliases, and other '
               + 'module state that `band-constants` gathers directly below the imports and '
               + 'sorts by name within each tier. A constant that reads a name bound later in '
               + 'the module stays below that binding rather than joining the band.',
    rule       : 'band-constants'
  },

  'banner comment': {
    aliases    : ['banner', 'banners', 'banner block', 'banner blocks'],
    definition : 'A banner comment is an own-line comment block whose lines are a rule of `=`, '
               + '`-`, `*`, `_`, `#`, `~`, `─`, `━`, or `═`, marking a section of a body. '
               + '`space-statements` keeps one blank line below it, and `alphabetize-siblings` '
               + 'sorts within the section it opens rather than across it.',
    families   : ['formatting', 'ordering'],
    rule       : 'space-statements'
  },

  'batch': {
    aliases    : ['batches', 'batched'],
    definition : 'A batch is a run of consecutive rules whose edits the pipeline applies to one '
               + 'buffer and parses once. It closes before a rule the registry places after one '
               + 'already in the batch, and before a rule whose edits overlap one already '
               + 'batched.',
    families   : ['engine'],
    href       : '/reference/pipeline-order#independent-rules-share-a-parse'
  },

  'blank line': {
    aliases    : ['blank-line', 'blank lines', 'blank-lines'],
    definition : 'A blank line is an empty line between two statements. `space-statements` '
               + 'sets the count between module-level definitions, class members, and import '
               + 'groups, keeps a descriptive comment block flush against the statement below '
               + 'it, and keeps one blank line below a banner comment block.',
    rule       : 'space-statements'
  },

  'cache': {
    aliases    : ['Cache', 'cached', 'caching'],
    definition : 'The cache is the on-disk store of results *Prose* keeps per user, so a '
               + 'repeat `prose check` or `prose format` run skips a file whose bytes, '
               + 'configuration, and *Prose* version have not changed. It is keyed by BLAKE3, '
               + 'capped by the `[cache] max-size-mib` LRU limit, bypassed for one run with '
               + '`--no-cache`, and emptied with `prose cache clean`.',
    families   : ['engine', 'cli'],
    href       : '/reference/cache'
  },

  'code-line-length': {
    definition : '`code-line-length` is the top-level config key setting the line budget for '
               + 'code. It defaults to **88**.',
    families   : ['cli', 'formatting'],
    href       : '/reference/configuration#top-level-keys'
  },

  'column': {
    aliases    : ['columns'],
    definition : 'A column is the character position an aligned token is padded out to, set '
               + 'by the widest row in its group. The alignment rules pad the space before '
               + '`=`, `:`, a comparison operator, the `import` keyword, or a trailing `#` '
               + 'so every row\'s token sits at that column.',
    families   : ['alignment'],
    href       : '/primitives/aligner'
  },

  'comprehension': {
    aliases    : [
      'comprehensions', 'list comprehension', 'dict comprehension', 'set comprehension'
    ],
    definition : 'A comprehension is a `[x for x in xs]`, `{k: v for ...}`, or `{x for ...}` '
               + 'expression building a list, dict, or set inline. `reflow-collections` joins '
               + 'one onto a single line where it fits, and `inlinable-bindings` never '
               + 'reports a name a comprehension binds.',
    families   : ['layout', 'lint'],
    href       : 'https://docs.python.org/3/tutorial/datastructures.html#list-comprehensions'
  },

  'count trigger': {
    aliases    : ['count-based trigger', 'count gate'],
    definition : 'A count trigger explodes a construct once its entry count exceeds a cap, '
               + 'whatever the width. `reflow-signatures` counts parameters against '
               + '`max-params` and `reflow-collections` counts dict entries against '
               + '`max-dict-entries`, and `false` turns either cap off. The count takes the '
               + 'place of the magic trailing comma Black and Ruff read.',
    rule       : 'reflow-collections'
  },

  'dataclass': {
    aliases    : ['dataclasses', 'dataclass field', 'dataclass fields'],
    definition : 'A dataclass is a class decorated with `@dataclass` whose typed fields become '
               + 'the positional parameters of the generated `__init__`. '
               + '`alphabetize-siblings` keeps those fields in source order for that reason, '
               + '`unsorted-positionals` reports a field run that is out of order, '
               + '`align-colons` aligns their annotation colons, and `align-equals` aligns '
               + 'their default `=` signs.',
    families   : ['ordering', 'alignment', 'lint'],
    href       : 'https://docs.python.org/3/library/dataclasses.html'
  },

  'decorator': {
    aliases    : ['decorators', 'decorated function', 'decorated functions'],
    definition : 'A decorator is an `@name` line above a function or class that wraps it at '
               + 'definition time. `alphabetize-siblings` sorts decorated functions together '
               + 'within each framework\'s decorator group, and `space-statements` keeps a '
               + 'decorator attached to its `def`.',
    families   : ['ordering', 'formatting'],
    href       : 'https://docs.python.org/3/glossary.html#term-decorator'
  },

  'docstring': {
    aliases    : ['docstrings'],
    definition : 'A docstring is the string literal that opens a module, class, or function '
               + 'body, whatever quotes it uses. `frame-docstrings` rewrites the quotes to '
               + '`"""`, `wrap-docstrings` rewraps a multi-line body, and `expand-docstrings` '
               + 'opens a single-line docstring onto its own lines.',
    families   : ['docs', 'engine'],
    href       : '/primitives/docstring'
  },

  'docstring-line-length': {
    definition : '`docstring-line-length` is the top-level config key setting the line budget '
               + 'for prose inside a docstring. It defaults to **76**.',
    families   : ['cli', 'docs'],
    href       : '/reference/configuration#top-level-keys'
  },

  'dunder': {
    aliases    : ['dunder name', 'dunder names', '__all__', '__slots__'],
    definition : 'A dunder is a name wrapped in double underscores (`__name__`, `__all__`, '
               + '`__init__`). `reassigned-constants` exempts dunder names, since the runtime '
               + 'writes them, `alphabetize-siblings` sorts dunder methods ahead of '
               + 'properties, private methods, and public methods in a class body, and '
               + '`prune-inert-imports` reads `__all__` as the public surface a module '
               + 'declares.',
    families   : ['ordering', 'lint', 'formatting']
  },

  'enum': {
    aliases    : ['Enum', 'enums', 'enum member', 'enum members'],
    definition : 'An enum is a subclass of `enum.Enum` whose body lists named constants. '
               + '`alphabetize-siblings` sorts the members by name, except where explicit '
               + 'integer or string values give them an order of their own.',
    families   : ['ordering'],
    href       : 'https://docs.python.org/3/library/enum.html'
  },

  'evaluation tier': {
    aliases    : ['evaluation tiers', 'tier', 'tiers', 'sub-band', 'sub-bands'],
    definition : 'An evaluation tier is the depth at which a module-level constant can be '
               + 'evaluated, where a constant reading no other member of the band sits in the '
               + 'first tier and one reading a member of a tier sits one tier below it. '
               + '`band-constants` opens a blank-line-separated sub-band per tier, up to the '
               + '`max-tiers` cap.',
    rule       : 'band-constants'
  },

  'explode': {
    aliases    : ['exploded', 'explodes', 'exploding'],
    definition : 'To explode a call, signature, collection, or `from` import is to rewrite it '
               + 'with one entry per line, the opening bracket ending the first line and the '
               + 'closing bracket on a line of its own. The layout rules explode a construct '
               + 'that outgrows the line budget or exceeds its count cap, and join one back '
               + 'onto a single line where it fits.',
    families   : ['layout']
  },

  'f-string': {
    aliases    : ['f-strings'],
    definition : 'An f-string is a string literal prefixed `f` whose `{}` fields interpolate '
               + 'expressions. Python assigns an f-string in docstring position no `__doc__`, '
               + 'so the docstring rules skip it. The layout rules never break a line inside '
               + 'a replacement field, because such a break parses only on Python 3.12 and '
               + 'later, leaving an over-wide interpolation for `line-overflow` to report, '
               + 'and `prefer-fstring` measures a conversion against the line budget before '
               + 'writing one.',
    families   : ['docs', 'formatting', 'layout', 'lint'],
    href       : 'https://docs.python.org/3/reference/lexical_analysis.html#f-strings'
  },

  'facet': {
    aliases    : ['facets'],
    definition : 'A facet is one key under a rule\'s table in `[tool.prose.rules]`, such as '
               + '`max-shift` on an alignment rule or `sort-dict-keys` on '
               + '`alphabetize-siblings`. Every rule carries `enabled`, and each further '
               + 'facet switches one behavior of that rule without turning off the rest.',
    families   : ['cli'],
    href       : '/reference/configuration#per-rule-facets'
  },

  'fix group': {
    aliases    : ['fix groups'],
    definition : 'A fix group is one set of edits a rule emits together, the inner `Vec<Edit>` '
               + 'of the `Vec<Vec<Edit>>` a rule\'s `apply` returns. The pipeline maps each '
               + 'group to one diagnostic and applies or suppresses it as a unit.',
    families   : ['engine'],
    href       : '/primitives/edit'
  },

  'fixture': {
    aliases    : ['fixtures', 'fixture pair'],
    definition : 'A fixture is an input file paired with the output *Prose* produces from it. '
               + 'Each rule page renders its fixtures as before-and-after Python, and the '
               + 'same files drive the snapshot tests inside the crate.',
    families   : ['engine']
  },

  'forward reference': {
    aliases    : ['forward references'],
    definition : 'A forward reference is an annotation naming a class or alias defined later '
               + 'in the file. `from __future__ import annotations` makes one safe on older '
               + 'runtimes, so `prune-inert-imports` removes that directive only when no '
               + 'annotation still needs it, and `alphabetize-siblings` never creates one, '
               + 'keeping a definition below any sibling it names at evaluation time.',
    families   : ['ordering'],
    rule       : 'prune-inert-imports'
  },

  'generation': {
    aliases    : ['generations'],
    definition : 'A generation is the cache subdirectory one build writes, named by a digest of '
               + 'the *Prose* version and the private entry-format version. A build reads only '
               + 'its own generation, deletes an older one it finds, and keeps its own ledger '
               + 'of the bytes its write-back runs produced.',
    families   : ['engine', 'cli'],
    href       : '/reference/cache#location'
  },

  'gitignore': {
    aliases    : ['.gitignore'],
    definition : '`.gitignore` is Git\'s exclusion file. The walker reads every `.gitignore` '
               + 'and `.ignore` on the way down a tree plus the user\'s global ignore file, so '
               + 'vendored dependencies and build output stay out of a run without further '
               + 'configuration.',
    families   : ['engine'],
    href       : '/primitives/walker#ignore-semantics'
  },

  'idempotent': {
    aliases    : ['idempotence', 'idempotency'],
    definition : 'A formatter is idempotent when a second run over its own output changes '
               + 'nothing. Every *Prose* rule keeps this property, so running `prose format` '
               + 'twice writes the same file the first run wrote.',
    families   : ['engine']
  },

  'implicit concatenation': {
    aliases    : ['adjacent literals', 'implicitly concatenated'],
    definition : 'Two string literals written side by side join into one value at compile '
               + 'time, so `("one " "two")` equals `"one two"`. `line-overflow` shows that '
               + 'form as a suggestion when an over-budget line\'s overflow sits inside one '
               + 'literal containing whitespace, and never writes it, since the split points '
               + 'would become text a later edit has to rewrap.',
    families   : ['layout', 'lint'],
    href       : 'https://docs.python.org/3/reference/lexical_analysis.html#string-literal-concatenation'
  },

  'import-line-length': {
    definition : '`import-line-length` is the top-level config key setting the line budget '
               + 'for a `from` import, which `reflow-imports` reads. It defaults to **120**, '
               + 'and `false` makes it follow `code-line-length`.',
    families   : ['cli', 'layout'],
    href       : '/reference/configuration#top-level-keys'
  },

  'independence table': {
    aliases    : ['independent', 'independence', 'shares a splice'],
    definition : 'The rule registry lists, beside each rule\'s dependencies, the earlier rules '
               + 'whose edits it can apply in the same pass and parse once. A pair joins that '
               + 'column only after the subset probe finds the batched splice matching the '
               + 'rule-by-rule result on every file both rules edit, and a reading of the later '
               + 'rule finds nothing it measures among what the earlier rule rewrites.',
    families   : ['engine'],
    href       : '/reference/pipeline-order#independent-rules-share-a-parse'
  },

  'kebab-case': {
    definition : 'Kebab-case is the lowercase, hyphen-joined form every rule slug takes '
               + '(`align-equals`, `inlinable-bindings`), the same spelling in CLI flags, '
               + 'config tables, suppression directives, and diagnostic output.',
    families   : ['engine', 'cli'],
    href       : '/primitives/rule-id'
  },

  'leading comment block': {
    aliases    : [
      'own-line comment', 'own-line comments', 'leading comment', 'leading comments',
      'own-line comment block'
    ],
    definition : 'A leading comment block is a run of own-line `#` comments directly above a '
               + 'statement. `space-statements` keeps a descriptive block flush against the '
               + 'statement below it and keeps one blank line below a banner block *(one '
               + 'whose line is a rule of `=`, `-`, `*`, `_`, `#`, `~`, `─`, `━`, or `═`)*, '
               + 'measuring the blank lines above from the topmost comment either way. A '
               + 'block that opens the file has no blank line above it and at most one '
               + 'below, kept where the author left one and dropped otherwise. When '
               + '`alphabetize-siblings` moves a statement, the orderer\'s `block_range` '
               + 'moves the block with it.',
    families   : ['ordering'],
    rule       : 'space-statements'
  },

  'lexical scope': {
    aliases    : ['lexical scopes', 'scope'],
    definition : 'A lexical scope is the region of a file within which a name refers to one '
               + 'binding. Python nests scopes by module, class, and function, and the '
               + 'binding analysis reads each once per `Source` to record every write and '
               + 'read.',
    families   : ['engine'],
    href       : '/primitives/binding-analysis'
  },

  'line budget': {
    aliases    : ['budget', 'budgets'],
    definition : 'The line budget is the width a rule fits a line within, read from '
               + '`code-line-length` for code, `docstring-line-length` for docstring prose, '
               + 'and `import-line-length` for a `from` import. A layout rule explodes a '
               + 'construct that outgrows its budget, and `line-overflow` reports a line no '
               + 'rule can bring within it.',
    families   : ['layout', 'cli'],
    href       : '/reference/configuration#top-level-keys'
  },

  'line continuation': {
    aliases    : ['backslash continuation'],
    definition : 'A line continuation is a trailing `\\` joining a physical line to the next, '
               + 'so one logical line spans two without a bracket. '
               + '`shed-backslash-continuations` removes it and lets the line break inside '
               + 'brackets instead. A backslash inside a docstring escapes a newline in the '
               + 'string\'s value rather than in the source line, so it stays.',
    families   : ['formatting'],
    rule       : 'shed-backslash-continuations'
  },

  'lint': {
    aliases    : ['Lint', 'lint violation', 'lint-only', 'linting'],
    definition : 'Lint is the category of rules whose diagnostics report a finding without '
               + 'rewriting it. `prose check` and `prose format` both report lints, and '
               + 'neither changes the source for one.',
    families   : ['lint']
  },

  'match': {
    aliases    : ['match statement', 'match-arm', 'match arms', 'match-case'],
    definition : 'A match is Python\'s structural pattern matching statement (PEP 634), each '
               + '`case Pattern: body` arm pairing a pattern with a body. `align-match-case` '
               + 'pads the space before the `:` across consecutive single-line arms so the '
               + 'colons share one column.',
    rule       : 'align-match-case'
  },

  'max-shift': {
    definition : '`max-shift` is the config key on each alignment rule limiting how much '
               + 'padding one row may take. It defaults to **16**. A positive `N` caps the '
               + 'gap between the widest and narrowest rows of a run, `0` forbids any '
               + 'padding, and `false` lifts the cap so a run of any width aligns on one '
               + 'column.',
    families   : ['alignment', 'cli'],
    href       : '/reference/configuration#per-rule-facets'
  },

  'module-level': {
    aliases    : ['module level', 'module-scope', 'module scope'],
    definition : 'Module-level names the outermost scope of a file, outside every class and '
               + 'function body. `reassigned-constants` reports module-level assignments '
               + 'only, and `space-statements` keeps two blank lines above every '
               + 'module-level `def` and `class`.',
    families   : ['engine', 'formatting', 'lint']
  },

  'NDJSON': {
    aliases    : ['ndjson', 'newline-delimited JSON'],
    definition : 'NDJSON is newline-delimited JSON. `prose check --output-format json` prints '
               + 'one record per line in this form, so an editor or a script can read '
               + 'diagnostics as they arrive rather than waiting for the whole document.',
    families   : ['cli'],
    href       : '/reference/output-formats#json'
  },

  'PEP 257': {
    aliases    : ['pep 257', 'PEP-257'],
    definition : 'PEP 257 is the docstring conventions PEP. It defines a docstring as the '
               + 'first statement of a module, class, or function body when that statement '
               + 'is a single string literal, which is exactly the shape the `docstring` '
               + 'walker matches.',
    families   : ['docs', 'engine'],
    href       : '/primitives/docstring#the-pep-257-definition'
  },

  'PEP 585': {
    aliases    : ['pep 585', 'PEP-585', 'builtin generic', 'builtin generics'],
    definition : 'PEP 585 is the builtin-generics PEP, from Python 3.9. It lets `list[int]` '
               + 'and `dict[str, int]` replace `List[int]` and `Dict[str, int]`, and '
               + '`modernize-annotations` rewrites to the builtin form on a project whose '
               + '`target-version` supports it.',
    rule       : 'modernize-annotations'
  },

  'PEP 604': {
    aliases    : ['pep 604', 'PEP-604', 'pipe-union', 'pipe-union syntax'],
    definition : 'PEP 604 is the union-operator PEP, from Python 3.10. It lets `X | Y` and '
               + '`T | None` replace `Union[X, Y]` and `Optional[T]`, and '
               + '`modernize-annotations` rewrites to the `|` form on a project whose '
               + '`target-version` supports it.',
    rule       : 'modernize-annotations'
  },

  'PEP 749': {
    aliases    : ['pep 749', 'PEP-749', 'deferred annotation', 'deferred annotations'],
    definition : 'PEP 749 is the deferred-annotation-evaluation PEP, landing in Python 3.14. '
               + 'Annotations are no longer evaluated when a definition runs, so '
               + '`from __future__ import annotations` does nothing there and '
               + '`prune-inert-imports` removes it on a project targeting 3.14 or later.',
    rule       : 'prune-inert-imports'
  },

  'pinned statement': {
    aliases    : ['pinned member', 'pinned constant', 'pinned definition'],
    definition : 'A pinned statement stays where the author put it while its siblings sort or '
               + 'band around it. A `# prose: keep` marker pins a dict literal or a dunder '
               + 'list, a reference that must resolve at evaluation time pins a definition '
               + 'below the sibling it names, and a `# noqa: E402` pins an import on its line.',
    families   : ['ordering']
  },

  'Pydantic': {
    aliases    : ['pydantic', 'Pydantic field', 'Pydantic fields'],
    definition : 'Pydantic is a data-validation library whose models declare typed fields in '
               + 'a class body. `alphabetize-siblings` sorts a `BaseModel`\'s fields with '
               + 'required fields before optional ones and keeps a `pydantic.dataclasses` '
               + 'field run in source order, because that decorator generates a positional '
               + 'constructor. `align-colons` aligns the annotation colons of either.',
    families   : ['ordering', 'alignment'],
    href       : 'https://docs.pydantic.dev/'
  },

  're-export': {
    aliases    : ['re-exports', 'reexport', 'reexports'],
    definition : 'A re-export is a name a module imports so that another module can import it '
               + 'from there. `prune-inert-imports` recognizes one by a name `__all__` lists, the '
               + '`x as x` alias form, a trailing `noqa`, or a name taken out of a private '
               + 'module, or a file-level unused-import pragma, and it reports rather than '
               + 'removes an unreferenced import where the file reads as a compatibility shim.',
    families   : ['formatting'],
    rule       : 'prune-inert-imports'
  },

  'reparse': {
    aliases    : ['reparses', 'reparsing'],
    definition : 'A reparse is the step the `Pipeline` runs between batches of independent '
               + 'rules, rebuilding the `Source` from their output. It re-parses only the '
               + 'statements those rules edited where it can and the whole file where it '
               + 'cannot. Each rule then reads a tree built from the text the rules before it '
               + 'wrote, so no rule sees a half-applied change, and the binding table carries '
               + 'into the new `Source` when every rule in the batch changed no binding.',
    families   : ['engine'],
    href       : '/primitives/pipeline'
  },

  'ruff format': {
    aliases    : ['ruff-format'],
    definition : '`ruff format` is Ruff\'s formatter subcommand. The integration page covers '
               + 'running it in the same project as `prose format`, including the '
               + '`pycodestyle` whitespace checks to turn off in Ruff\'s linter.',
    families   : ['engine'],
    href       : '/integrations/ruff'
  },

  'ruff_python_parser': {
    definition : '`ruff_python_parser` is the Astral parser crate that builds the tree inside '
               + 'each `Source`. The reparse between batches rebuilds that tree from the text '
               + 'the earlier rules wrote.',
    families   : ['engine']
  },

  'run': {
    definition : 'A run is a stretch of consecutive lines a rule reads as one unit, such as '
               + 'the assignments an alignment rule pads to one column or the constants '
               + '`band-constants` gathers. A blank line, a comment line, or a line of '
               + 'another kind ends it. The word also names one execution of *Prose*, as in '
               + '*a `prose format` run*, in its ordinary sense.',
    families   : ['alignment', 'engine'],
    href       : '/primitives/aligner'
  },

  'Severity': {
    aliases    : ['severity'],
    definition : 'Severity is the kind of change a diagnostic carries. `Format` marks a '
               + 'rewrite `prose format` writes and `prose check` reports as pending, whereas '
               + '`Lint` marks a finding no rule rewrites.',
    families   : ['engine']
  },

  'settle': {
    aliases    : ['settled', 'settles', 'settling'],
    definition : 'A file has settled when a second `prose format` run would change nothing in '
               + 'it. Every run ends in that state, because each rule measures the columns it '
               + 'writes rather than the columns a later rule will move. The guarantee holds '
               + 'for any subset of rules a project enables, so a `--select` run also settles '
               + 'in one pass.',
    families   : ['engine'],
    href       : '/reference/pipeline-order#every-subset-settles'
  },

  'span trigger': {
    aliases    : ['span-based trigger', 'row-span trigger'],
    definition : 'A span trigger explodes a construct that still spans several lines after '
               + 'every bracket inside it that could close has closed, whatever its entry '
               + 'count or joined width. `reflow-calls` reads arguments this way and '
               + '`reflow-signatures` reads parameters, so a construct the author wrote '
               + 'across lines takes the one-per-line form a long one takes. An entry aligned '
               + 'under its own opening bracket is left alone, because that alignment would '
               + 'have nothing to align to once its line moved.',
    rule       : 'reflow-calls'
  },

  'splice': {
    aliases    : ['splices', 'spliced'],
    definition : 'A splice is the step that applies a batch\'s edits to the buffer and rebuilds '
               + 'the tree, re-parsing only the statements the edits touched where it can and '
               + 'the whole file where it cannot.',
    families   : ['engine'],
    href       : '/primitives/pipeline'
  },

  'stdin mode': {
    aliases    : ['--stdin', 'stdin'],
    definition : 'Stdin mode reads one file from standard input and writes to standard '
               + 'output, skipping the file walker altogether, so an editor or a script can '
               + 'run *Prose* without touching the disk.',
    families   : ['cli']
  },

  'strip-stranded-padding': {
    aliases    : ['singleton rule', 'singleton rules', 'stranded padding', 'stranded'],
    definition : '`strip-stranded-padding` removes padding that aligns with nothing. It '
               + 'strips the space before a `:` in a group with no column to share, either a '
               + 'one-member group or a group whose colons all sit on one line, and it '
               + 'removes the spaces just inside a bracket, so a one-key dict reads as plain '
               + 'code and `int(a )` becomes `int(a)`.',
    rule       : 'strip-stranded-padding'
  },

  'structured section': {
    aliases    : [
      'structured sections', 'Args block', 'Args section', 'Returns section', 'Raises section'
    ],
    definition : 'A structured section is a docstring section such as `Args:`, `Returns:`, '
               + 'or `Raises:` that reads as a table rather than prose. `wrap-docstrings` '
               + 'wraps its prose lines to `code-line-length` by default, and wraps each '
               + '`name: description` entry to `docstring-line-length` with a hanging indent '
               + 'at the column the description starts on.',
    families   : ['alignment'],
    rule       : 'wrap-docstrings'
  },

  'target-version': {
    aliases    : ['target version'],
    definition : '`target-version` is the top-level config key naming the Python version the '
               + 'project runs on. The version-gated rules read it, and with it unset no '
               + 'version-dependent rewrite fires.',
    families   : ['cli', 'lint'],
    href       : '/reference/configuration#top-level-keys'
  },

  'type-alias': {
    aliases    : ['type aliases', 'bare type alias', 'alias value'],
    definition : 'A type alias is a name bound to an existing type rather than to a value '
               + '(`Interval = int | float`, `Pen = Turtle`). `miscased-constants` never '
               + 'renames one, telling an alias from a constant by the value\'s form, by what '
               + 'the file assigns to its base name, and by how the module uses the name, and '
               + 'it leaves alone any name it cannot classify. `band-constants` sorts the '
               + 'same statements into the alias sub-band.',
    families   : ['lint', 'ordering']
  },

  'TYPE_CHECKING': {
    aliases    : ['typing.TYPE_CHECKING', 'if TYPE_CHECKING'],
    definition : '`TYPE_CHECKING` is a `typing` flag that reads `False` at runtime and `True` '
               + 'to a type checker, so an `if TYPE_CHECKING:` block holds imports only a '
               + 'type checker needs. `reassigned-constants` exempts a binding inside such '
               + 'a block.',
    families   : ['lint'],
    href       : 'https://docs.python.org/3/library/typing.html#typing.TYPE_CHECKING'
  },

  'TypedDict': {
    aliases    : ['typeddict'],
    definition : 'A `TypedDict` is a `typing.TypedDict` subclass declaring the key and value '
               + 'types of a dict. It takes no positional field arguments, so '
               + '`alphabetize-siblings` sorts its fields the way it sorts a `BaseModel`\'s.',
    families   : ['ordering', 'alignment'],
    href       : 'https://docs.python.org/3/library/typing.html#typing.TypedDict'
  },

  'unstable output': {
    aliases    : ['unstable rewrite', 'unsettled rewrite'],
    definition : 'Unstable output is a rewrite a second run would change again, a defect in '
               + '*Prose* rather than in the file. `prose format` re-applies its enabled '
               + 'rules to each file it rewrote and reports any file that still changes, '
               + 'naming the smallest rule subset that reproduces it and the command that '
               + 'replays it. The rewrite is still written and the exit code ignores the '
               + 'notice, whereas `prose check --validate` fails on the same finding, and '
               + '`report-unstable-output = false` turns the notice off.',
    families   : ['engine'],
    href       : '/reference/cli#unstable-output'
  },

  'walrus operator': {
    aliases    : ['walrus', 'walrus assignment'],
    definition : 'The walrus operator is Python\'s assignment expression `:=` (PEP 572). '
               + '`align-equals` never aligns it, so a walrus inside a condition or '
               + 'comprehension stays as written.',
    families   : ['alignment']
  },

  'workflow command': {
    aliases    : ['workflow commands', 'workflow-command annotation'],
    definition : 'A workflow command is GitHub Actions\' annotation syntax '
               + '(`::warning file=...,line=...::message`). `--output-format github` prints '
               + 'one per diagnostic, and GitHub renders each as an annotation on the PR '
               + 'diff.',
    families   : ['cli'],
    href       : '/reference/output-formats#github'
  }
}
