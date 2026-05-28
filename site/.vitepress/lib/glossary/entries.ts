import type { GlossaryFamily } from '../shared/registries'

export interface GlossaryEntry {
  aliases   ?: readonly string[]
  definition : string
  families   : readonly [GlossaryFamily, ...GlossaryFamily[]]
  href      ?: string
}

export const glossary: Record<string, GlossaryEntry> = {
  '# fmt: off': {
    aliases   : ['# fmt: on'],
    definition: '`# fmt: off` and `# fmt: on` are block markers that preserve the exact source '
              + 'layout of code between them by disabling every rewriting rule. Inline '
              + 'comments on the same line are recognized as the marker.',
    families  : ['formatting', 'engine'],
    href      : '/usage/suppression#block-markers'
  },

  '# fmt: skip': {
    definition: '`# fmt: skip` is a line-level marker that exempts the statement it sits on '
              + 'from every rewriting rule, without needing surrounding block markers.',
    families  : ['formatting', 'engine'],
    href      : '/usage/suppression#line-markers'
  },

  '# prose: ignore': {
    aliases   : ['# prose: ignore[...]'],
    definition: '`# prose: ignore` is a per-line directive that suppresses specific lint '
              + 'diagnostics. The bracketed form names the rule slugs to silence, whereas the '
              + 'bare form silences every lint on that line.',
    families  : ['lint', 'engine'],
    href      : '/usage/suppression#lint-directives'
  },

  '--ignore': {
    definition: '`--ignore` is a CLI flag that disables the named rules for a single '
              + 'invocation. The flag is repeatable, and pairs with `--select` to scope a run.',
    families  : ['cli'],
    href      : '/usage/quick-start#subset-the-active-rules'
  },

  '--no-cache': {
    definition: '`--no-cache` is a CLI flag that bypasses the user-level cache for a single '
              + 'invocation of `prose check` or `prose format`. The flag overrides the '
              + 'configured `[tool.prose.cache] enabled` value.',
    families  : ['cli'],
    href      : '/reference/cache#no-cache'
  },

  '--select': {
    definition: '`--select` is a CLI flag that restricts a run to the named rules. The flag is '
              + 'repeatable, and pairs with `--ignore` to subtract from the active set.',
    families  : ['cli'],
    href      : '/usage/quick-start#subset-the-active-rules'
  },

  '--verbose': {
    definition: '`--verbose` is a global CLI flag that prints a one-line cache hit/miss '
              + 'summary to stderr at the end of each `prose check` or `prose format` run. '
              + 'The flag writes `cache: bypassed` when the cache is disabled via '
              + '`--no-cache` or `[tool.prose.cache] enabled = false`.',
    families  : ['cli'],
    href      : '/reference/cache#hit-miss-telemetry'
  },

  'AST': {
    aliases   : ['abstract syntax tree'],
    definition: 'An AST is the parsed-program tree produced by `ruff_python_parser`. *Prose* '
              + 'bundles it inside `Source` and reparses it between rules so each rule reads '
              + 'against the post-rewrite tree.',
    families  : ['engine'],
    href      : '/primitives/source'
  },

  'BindingAnalysis': {
    aliases   : ['binding analysis', 'binding map', 'name bindings', 'binding', 'bindings'],
    definition: '`BindingAnalysis` is a per-`Source` table indexing every write and read of '
              + 'every name in every lexical scope. The `single-use-variables` rule consumes '
              + 'it.',
    families  : ['engine', 'lint'],
    href      : '/primitives/binding-analysis'
  },

  'BLAKE3': {
    definition: 'BLAKE3 is the cryptographic hash function *Prose* uses to derive [[Cache]] '
              + 'keys. The key digests the source bytes, the canonical TOML serialization of '
              + 'the active configuration, and the *Prose* version, so any meaningful change '
              + 'to any input produces a different key.',
    families  : ['engine', 'cli'],
    href      : '/reference/cache#key-shape'
  },

  'Diagnostic': {
    aliases   : ['diagnostic', 'diagnostics', 'lint diagnostic'],
    definition: 'A `Diagnostic` is the structured report a rule emits when it detects a '
              + 'pattern. It carries a severity, wherein `AutoFix` rewrites source under '
              + '`prose format` and `Lint` only surfaces.',
    families  : ['engine', 'lint']
  },

  'Pipeline': {
    aliases   : ['pipeline'],
    definition: 'The `Pipeline` orchestrates the rule loop against a `Source`, reparses '
              + 'between rules, and returns the final source plus diagnostics.',
    families  : ['engine'],
    href      : '/primitives/pipeline'
  },

  'Ruff': {
    aliases   : ['ruff'],
    definition: 'Ruff is Astral\'s Python linter and formatter. *Prose* composes cleanly with '
              + 'it when both are in the toolchain, leaving token-level normalization to '
              + '`ruff` and layout-level legibility to *Prose*.',
    families  : ['engine'],
    href      : '/integrations/ruff'
  },

  'RuleId': {
    aliases   : ['rule id', 'rule IDs'],
    definition: 'A `RuleId` is the canonical kebab-case slug identifying each registered rule '
              + 'across CLI flags, config tables, suppression directives, and diagnostic '
              + 'output.',
    families  : ['engine', 'cli'],
    href      : '/primitives/rule-id'
  },

  'Source': {
    definition: '`Source` is the parsed-text wrapper bundling original text, AST, token '
              + 'stream, line index, and suppression map. Every rule reads through this value.',
    families  : ['engine'],
    href      : '/primitives/source'
  },

  'SuppressionMap': {
    aliases   : [
      'suppression map', 'suppression directive', 'suppression directives', 'suppression'
    ],
    definition: '`SuppressionMap` is the per-`Source` index of `# fmt: off` / `# fmt: skip` / '
              + '`# yapf` / `# prose: ignore[...]` directives, consulted at the edit-emission '
              + 'boundary.',
    families  : ['engine', 'formatting', 'lint'],
    href      : '/primitives/suppression-map'
  },

  'alignment group': {
    aliases   : ['alignment groups', 'group', 'singleton group', 'singleton groups'],
    definition: 'An alignment group is a run of consecutive members at the same indentation '
              + 'that share an alignment target. Blank lines, comment lines, and non-member '
              + 'statements reset the run, so each contiguous group resolves independently.',
    families  : ['alignment'],
    href      : '/primitives/aligner'
  },

  'annotation': {
    aliases   : ['annotations', 'type annotation', 'type annotations'],
    definition: 'An annotation is a `name: Type` declaration on a function parameter, return '
              + 'value, or variable. Type checkers and version-gated rules like '
              + '`legacy-union-syntax` and `unused-future-annotations` read it.',
    families  : ['lint', 'alignment'],
    href      : 'https://docs.python.org/3/glossary.html#term-annotation'
  },

  'applicability': {
    definition: 'Applicability is the Ruff-shared confidence level on an auto-fix\'s `fix` '
              + 'payload. `safe` means the rewrite preserves runtime semantics and an editor '
              + 'can apply it without prompting, whereas `unsafe` and `display` exist in the '
              + 'schema for forward compatibility.',
    families  : ['cli', 'engine'],
    href      : '/reference/output-formats#json'
  },

  'atomic': {
    aliases   : ['atomic literal', 'atomic literals'],
    definition: 'An atomic is a simple, indivisible code element (integer, float, string, '
              + 'single name) that `collection-layout` can safely keep on one line without '
              + 'readability loss.',
    families  : ['formatting'],
    href      : '/rules/collection-layout'
  },

  'auto-fix': {
    aliases   : ['auto-fixes', 'auto-fixing', 'Auto-Fix'],
    definition: 'Auto-fix is the rule category whose diagnostics rewrite source under `prose '
              + 'format` and surface as `Severity::AutoFix` under `prose check`.',
    families  : ['formatting']
  },

  'blank line': {
    aliases   : ['blank-line', 'blank lines', 'blank-lines'],
    definition: 'A blank line is an empty line separating logical units. *Prose* enforces '
              + 'blank-line counts between module-level definitions, class members, and import '
              + 'groups per the `blank-lines` rule, and binds description-shaped own-line '
              + 'comment blocks tight against the following statement while leaving '
              + 'banner-shaped blocks separated by 1 blank line below.',
    families  : ['formatting'],
    href      : '/rules/blank-lines'
  },

  'cache': {
    aliases   : ['Cache', 'cached', 'caching'],
    definition: 'The user-level on-disk cache *Prose* keeps for repeat `prose check` and '
              + '`prose format` runs. Keyed by [[BLAKE3]] over '
              + '`(source ++ config ++ prose_version)`, capped by the LRU eviction the '
              + '`[tool.prose.cache] max-size-mib` knob configures, bypassable per invocation '
              + 'with `--no-cache`, and clearable with `prose cache clean`.',
    families  : ['engine', 'cli'],
    href      : '/reference/cache'
  },

  'code-line-length': {
    definition: '`code-line-length` is the top-level config key for the line budget consumed '
              + 'by code-shaped rules. It defaults to **88**.',
    families  : ['cli', 'formatting'],
    href      : '/reference/configuration#top-level-keys'
  },

  'comprehension': {
    aliases   : [
      'comprehensions', 'list comprehension', 'dict comprehension', 'set comprehension'
    ],
    definition: 'A comprehension is one of Python\'s `[x for x in xs]`, `{k: v for ...}`, or '
              + '`{x for ...}` literal forms that build a list, dict, or set inline. '
              + '`collection-layout` keeps them on one line when they fit, and their bound '
              + 'targets sit outside `single-use-variables`.',
    families  : ['formatting', 'lint'],
    href      : 'https://docs.python.org/3/tutorial/datastructures.html#list-comprehensions'
  },

  'dataclass': {
    aliases   : ['dataclasses', 'dataclass field', 'dataclass fields'],
    definition: 'A dataclass is a class decorated with `@dataclass` whose body lists typed '
              + 'field declarations. `alphabetize` reorders the fields (required before '
              + 'optional), `align-colons` aligns their annotation colons, and `align-equals` '
              + 'aligns their default-value `=` signs.',
    families  : ['ordering', 'alignment'],
    href      : 'https://docs.python.org/3/library/dataclasses.html'
  },

  'decorator': {
    aliases   : ['decorators', 'decorated function', 'decorated functions'],
    definition: 'A decorator is an `@name` prefix attached to a function or class definition '
              + 'that wraps it at definition time. `alphabetize` sorts decorated functions '
              + 'together inside framework-decorator groups, and `blank-lines` keeps each '
              + 'decorator attached to its `def`.',
    families  : ['ordering', 'formatting'],
    href      : 'https://docs.python.org/3/glossary.html#term-decorator'
  },

  'docstring': {
    aliases   : ['docstrings', 'triple-quoted docstring'],
    definition: 'A docstring is a triple-quoted string literal placed as the first statement '
              + 'in a module, class, or function. *Prose* rewraps multi-line bodies under '
              + '`docstring-wrap` and gates single-line shapes under '
              + '`no-single-line-docstrings`.',
    families  : ['docs', 'engine'],
    href      : '/primitives/docstring'
  },

  'docstring-line-length': {
    definition: '`docstring-line-length` is the top-level config key for the description-prose '
              + 'budget inside docstrings. It defaults to **76**.',
    families  : ['cli', 'docs'],
    href      : '/reference/configuration#top-level-keys'
  },

  'dunder': {
    aliases   : ['dunder name', 'dunder names', '__all__', '__slots__'],
    definition: 'A dunder is the Python convention for names wrapped in double underscores '
              + '(`__name__`, `__all__`, `__init__`). `loose-constants` treats them as runtime '
              + 'sentinels, and `alphabetize` treats them as ordering anchors that surface '
              + 'before properties and privates inside a class body.',
    families  : ['ordering', 'lint']
  },

  'enum': {
    aliases   : ['Enum', 'enums', 'enum member', 'enum members'],
    definition: 'An enum is a subclass of `enum.Enum` whose body lists named constants. '
              + '`alphabetize` sorts the members, except when they carry explicit integer or '
              + 'string values that encode ordering.',
    families  : ['ordering'],
    href      : 'https://docs.python.org/3/library/enum.html'
  },

  'f-string': {
    aliases   : ['f-strings'],
    definition: 'An f-string is a Python string literal prefixed `f"..."` that interpolates '
              + 'expressions inside `{}` placeholders. The `docstring` walker skips f-string '
              + 'and other concatenated forms, so only plain triple-quoted string literals '
              + 'count as docstrings.',
    families  : ['formatting', 'docs'],
    href      : 'https://docs.python.org/3/reference/lexical_analysis.html#f-strings'
  },

  'fixture': {
    aliases   : ['fixtures', 'fixture pair'],
    definition: 'A fixture is an input-and-output pair that pins a rule\'s behavior. Each rule '
              + 'page renders fixtures inline as side-by-side before-and-after Python '
              + 'snippets, and the same files drive snapshot testing inside the crate.',
    families  : ['engine']
  },

  'forward reference': {
    aliases   : ['forward references'],
    definition: 'A forward reference is an annotation that names a class or alias defined '
              + 'later in the file. The `from __future__ import annotations` directive made '
              + 'these safe on older Python runtimes, and `unused-future-annotations` removes '
              + 'the directive when no annotation needs the forward reference.',
    families  : ['lint'],
    href      : '/rules/unused-future-annotations'
  },

  'gitignore': {
    aliases   : ['.gitignore'],
    definition: '`.gitignore` is the standard Git exclusion file. *Prose*\'s walker honors '
              + '`.gitignore` and `.ignore` files at every level of the walked tree plus the '
              + 'user\'s global gitignore, so vendored dependencies and build artifacts stay '
              + 'out of the run automatically.',
    families  : ['engine'],
    href      : '/primitives/walker#ignore-semantics'
  },

  'idempotent': {
    aliases   : ['idempotence', 'idempotency'],
    definition: 'A second `prose format` run against `prose`-formatted source produces no '
              + 'further edits. Every rule preserves this property, so re-running the '
              + 'formatter never thrashes the source.',
    families  : ['engine']
  },

  'kebab-case': {
    definition: 'Kebab-case is the lowercase-with-hyphens naming convention *Prose* uses for '
              + 'every rule slug (`align-equals`, `single-use-variables`). The form is '
              + 'canonical across CLI flags, config tables, suppression directives, and '
              + 'diagnostic output.',
    families  : ['engine', 'cli'],
    href      : '/primitives/rule-id'
  },

  'leading comment block': {
    aliases   : [
      'own-line comment', 'own-line comments', 'leading comment', 'leading comments',
      'own-line comment block'
    ],
    definition: 'A leading comment block is a run of own-line `#` comments sitting directly '
              + 'above a statement. `blank-lines` binds description-shaped blocks tight '
              + 'against the following statement and keeps 1 blank line below banner-shaped '
              + 'blocks *(any line of which is a decorative rule of `=`, `-`, `*`, `_`, '
              + '`#`, or `~`)*, with the canonical above-gap measured from the topmost '
              + 'comment either way. The orderer primitive\'s `block_range` carries the '
              + 'block with its item when reordering siblings.',
    families  : ['formatting', 'ordering'],
    href      : '/rules/blank-lines'
  },

  'lexical scope': {
    aliases   : ['lexical scopes', 'scope'],
    definition: 'A lexical scope is the textual region of source code in which a name resolves '
              + 'to a given binding. Python scopes nest by module, class, and function, and '
              + '`binding-analysis` walks them once per `Source` to index every write and '
              + 'read.',
    families  : ['engine'],
    href      : '/primitives/binding-analysis'
  },

  'lint': {
    aliases   : ['Lint', 'lint violation', 'lint-only', 'linting'],
    definition: 'Lint is the rule category whose diagnostics surface as `Severity::Lint` '
              + 'without rewriting source. *Prose* always inspects them, but never modifies '
              + 'the source.',
    families  : ['lint']
  },

  'match': {
    aliases   : ['match statement', 'match-arm', 'match arms', 'match-case'],
    definition: 'A match is Python\'s structural-pattern-matching statement (PEP 634). Each '
              + '`case Pattern: body` arm pairs a pattern with a body, and `match-case-align` '
              + 'shares a column for the post-pattern `:` separator across consecutive '
              + 'single-expression arms.',
    families  : ['alignment'],
    href      : '/rules/match-case-align'
  },

  'max-shift': {
    definition: '`max-shift` is the per-alignment-rule config key capping per-line padding. It '
              + 'defaults to **8**, and groups whose widest member exceeds the cap fall back '
              + 'to `max-shift-policy`.',
    families  : ['alignment', 'cli'],
    href      : '/reference/configuration#per-rule-knobs'
  },

  'max-shift-policy': {
    definition: '`max-shift-policy` decides how an alignment group overflowing `max-shift` '
              + 'resolves. `split` partitions the group, `drop` excludes the widest members, '
              + 'and `skip` leaves the whole group unaligned.',
    families  : ['alignment', 'cli'],
    href      : '/reference/configuration#per-rule-knobs'
  },

  'module-level': {
    aliases   : ['module level', 'module-scope', 'module scope'],
    definition: 'Module-level names the outermost lexical scope of a Python file, sitting '
              + 'outside any class or function body. `loose-constants` fires only on '
              + 'module-level assignments, and `blank-lines` reserves two blanks above every '
              + 'module-level `def` and `class`.',
    families  : ['engine', 'formatting', 'lint']
  },

  'NDJSON': {
    aliases   : ['ndjson', 'newline-delimited JSON'],
    definition: 'NDJSON is newline-delimited JSON. `prose check --output-format json` emits '
              + 'one record per line in this shape, so editors and tooling can stream '
              + 'diagnostics without buffering the whole document.',
    families  : ['cli'],
    href      : '/reference/output-formats#json'
  },

  'PEP 257': {
    aliases   : ['pep 257', 'PEP-257'],
    definition: 'PEP 257 is the docstring conventions PEP. It defines a docstring as the first '
              + 'body statement of a module, class, or function when that statement is a '
              + 'single string literal expression, and the `docstring` walker matches this '
              + 'shape exactly.',
    families  : ['docs', 'engine'],
    href      : '/primitives/docstring#the-pep-257-definition'
  },

  'PEP 604': {
    aliases   : ['pep 604', 'PEP-604', 'pipe-union', 'pipe-union syntax'],
    definition: 'PEP 604 is the pipe-union syntax PEP (Python 3.10+). It lets `X | Y` and `T | '
              + 'None` replace `Union[X, Y]` and `Optional[T]` at the type level, and '
              + '`legacy-union-syntax` surfaces the legacy `typing` forms on projects whose '
              + '`target-version` allows the pipe form.',
    families  : ['lint'],
    href      : '/rules/legacy-union-syntax'
  },

  'PEP 749': {
    aliases   : ['pep 749', 'PEP-749', 'deferred annotation', 'deferred annotations'],
    definition: 'PEP 749 is the deferred-annotation-evaluation PEP, landing in Python 3.14. '
              + 'The runtime no longer evaluates annotations eagerly for typing-only code, so '
              + '`from __future__ import annotations` becomes redundant and '
              + '`unused-future-annotations` removes it on 3.14+.',
    families  : ['lint'],
    href      : '/rules/unused-future-annotations'
  },

  'Pydantic': {
    aliases   : ['pydantic', 'Pydantic field', 'Pydantic fields'],
    definition: 'Pydantic is a widely used data-validation library whose models declare typed '
              + 'fields in the class body. `alphabetize` sorts those fields with required '
              + 'before optional, and `align-colons` aligns the annotation colons across the '
              + 'field block.',
    families  : ['ordering', 'alignment'],
    href      : 'https://docs.pydantic.dev/'
  },

  'reparse': {
    aliases   : ['reparses', 'reparsing'],
    definition: 'Reparse names the `Source::reparse` step the `Pipeline` runs between rules. '
              + 'Each rule reads a settled AST built from the post-rewrite text, so no rule '
              + 'observes another rule\'s half-applied state.',
    families  : ['engine'],
    href      : '/primitives/pipeline'
  },

  'ruff format': {
    aliases   : ['ruff-format'],
    definition: '`ruff format` is Ruff\'s formatter subcommand. When a project pairs it with '
              + '*Prose*, Ruff runs first to settle line wraps, quote normalization, '
              + 'indentation, and blank-line discipline, after which `prose format` runs '
              + 'against the settled tokens.',
    families  : ['engine'],
    href      : '/integrations/ruff'
  },

  'ruff_python_parser': {
    definition: '`ruff_python_parser` is the Astral parser crate *Prose* consumes to produce '
              + 'the AST inside each `Source`. Reparsing between rules guarantees every rule '
              + 'reads against the post-rewrite tree.',
    families  : ['engine']
  },

  'Severity': {
    aliases   : ['severity'],
    definition: 'Severity is a diagnostic\'s emission kind. `AutoFix` rewrites source under '
              + '`prose format` and surfaces as a pending change under `prose check`, whereas '
              + '`Lint` only reports and never rewrites.',
    families  : ['engine']
  },

  'singleton rule': {
    aliases   : ['singleton rules'],
    definition: 'The singleton rule drops alignment padding when a group resolves to a single '
              + 'member, so a one-key dict reads as plain code.',
    families  : ['alignment'],
    href      : '/rules/singleton-rule'
  },

  'stdin mode': {
    aliases   : ['--stdin', 'stdin'],
    definition: 'Stdin mode is the CLI shape that reads a single source from standard input '
              + 'and writes to standard output. It bypasses the filesystem walker entirely, so '
              + 'editors and pipelines drive *Prose* without touching disk.',
    families  : ['cli']
  },

  'structured section': {
    aliases   : [
      'structured sections', 'Args block', 'Args section', 'Returns section', 'Raises section'
    ],
    definition: 'A structured section is a docstring section like `Args:`, `Returns:`, or '
              + '`Raises:` that reads as a code-shaped table rather than prose. '
              + '`docstring-wrap` budgets these against `code-line-length` by default, so '
              + 'argument lines align with surrounding code.',
    families  : ['docs', 'alignment'],
    href      : '/rules/docstring-wrap'
  },

  'target-version': {
    aliases   : ['target version'],
    definition: '`target-version` is the top-level config key naming the Python runtime the '
              + 'project ships to. Version-gated rules consume it, and leaving it unset means '
              + 'no version-dependent rewrites fire.',
    families  : ['cli', 'lint'],
    href      : '/reference/configuration#top-level-keys'
  },

  'TYPE_CHECKING': {
    aliases   : ['typing.TYPE_CHECKING', 'if TYPE_CHECKING'],
    definition: '`TYPE_CHECKING` is a `typing` flag that is `False` at runtime and `True` to '
              + 'type checkers, used inside `if TYPE_CHECKING:` blocks to guard '
              + 'import-only-for-typing code. `loose-constants` exempts bindings declared '
              + 'inside the block.',
    families  : ['lint'],
    href      : 'https://docs.python.org/3/library/typing.html#typing.TYPE_CHECKING'
  },

  'TypedDict': {
    aliases   : ['typeddict'],
    definition: 'A `TypedDict` is a `typing.TypedDict` subclass declaring a dict\'s '
              + 'key-to-value-type contract. `alphabetize` sorts its fields the same way it '
              + 'sorts `dataclass` and Pydantic fields.',
    families  : ['ordering', 'alignment'],
    href      : 'https://docs.python.org/3/library/typing.html#typing.TypedDict'
  },

  'walrus operator': {
    aliases   : ['walrus', 'walrus assignment'],
    definition: 'The walrus operator is Python\'s assignment expression `:=` (PEP 572). '
              + '`align-equals` treats it as a non-member, so a walrus inside a condition or '
              + 'comprehension never enters an alignment group.',
    families  : ['alignment']
  },

  'workflow command': {
    aliases   : ['workflow commands', 'workflow-command annotation'],
    definition: 'A workflow command is GitHub Actions\' inline-annotation syntax (`::warning '
              + 'file=...,line=...::message`). The `--output-format github` shape emits one '
              + 'workflow command per diagnostic, which GitHub renders as a check-run '
              + 'annotation on the PR diff.',
    families  : ['cli'],
    href      : '/reference/output-formats#github'
  }
}
