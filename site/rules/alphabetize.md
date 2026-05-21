---
category : auto-fix
domain   : ordering
caption  : "*Prose* alphabetizes import siblings, dict-key blocks, and dataclass field runs."
related  : [align-colons, align-imports, bare-import-allowlist, blank-lines]
---

# alphabetize

A reader who already knows the codebase carries a **mental map** of where things live. When sibling members within a class, an enum, a dataclass, or a function call sit in arrival order, every reader builds a **different map**, which slows each new reader's first read. `alphabetize` gives everyone the **same landmarks**, with classes ordered alphabetically inside a module, methods ordered inside a class body (*dunders first, then properties, then private, then public*), enum members ordered, dataclass and Pydantic fields ordered (*required before optional*), function parameters with defaults ordered, keyword arguments at call sites ordered, and `from` imports ordered within each block.

The rule fires on siblings whose order does not carry meaning. It leaves alone every surface where ordering is load-bearing (*positional-only parameters before the `/` separator, enum members with explicit integer or string values, tuple-unpacking targets bound to positional results*). Pair with [[align-imports]] to align the `import` keyword across the freshly-sorted block, with [[align-colons]] to align dataclass-field annotations after the sort, and with [[blank-lines]] for the blank-line discipline around class members.

## Configuration

<RuleConfigTable preset="toggle" />

The ordering itself follows fixed per-construct conventions without per-project knobs. Method groups follow the dunders-properties-privates-publics rhythm. Pydantic fields follow required-then-optional. Imports sort alphabetically within each block.

## The Canonical Case

Classes inside a module sort alphabetically, giving every reader the same first-pass landmarks.

<Fixture rule="alphabetize" case="classes" />

## More Examples

<Fixture rule="alphabetize" case="class_with_branched_body" title="Methods Follow the Dunders → Properties → Private → Public Rhythm" />

<Fixture rule="alphabetize" case="dataclass" title="Dataclass Fields Sort, Required before Optional" />

<Fixture rule="alphabetize" case="from_imports" title="`from` Imports Sort Alphabetically inside Their Block" />

<Fixture rule="alphabetize" case="bare_imports" title="Bare Imports Sort Alphabetically Too" />

<Fixture rule="alphabetize" case="annotated_field_default" title="Field Defaults Are Preserved Across the Reorder" />

<Fixture rule="alphabetize" case="async_compound" title="Async Methods Sort Beside Their Sync Siblings" />

<Fixture rule="alphabetize" case="dict_keep_marker" title="`# prose: keep` on a Dict Preserves Source Order" />

<Fixture rule="alphabetize" case="enum" title="Enum Members Sort Alphabetically" />

<Fixture rule="alphabetize" case="kwargs" title="Keyword Arguments at Call Sites Sort" />

<Fixture rule="alphabetize" case="params" title="Function Parameters with Defaults Sort" />

<Fixture rule="alphabetize" case="pydantic" title="Pydantic Fields Sort, Required before Optional" />

<Fixture rule="alphabetize" case="namedtuple" title="`namedtuple` Fields Sort" />

<Fixture rule="alphabetize" case="typeddict" title="`TypedDict` Fields Sort" />

<Fixture rule="alphabetize" case="dunder_all" title="`__all__` Sorts Alphabetically" />

<Fixture rule="alphabetize" case="dunder_slots" title="`__slots__` Sorts Alphabetically" />

<Fixture rule="alphabetize" case="sets" title="Set Literals Sort" />

<Fixture rule="alphabetize" case="dict_keys" title="Dictionary Keys Sort When Keys Are String Literals" />

<Fixture rule="alphabetize" case="framework_decorators" title="Decorated Functions Sort Together inside Framework Groups" />

## Related

<RelatedRulesInline />

