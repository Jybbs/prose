---
caption : "Reports a run of positionally-bound names that is out of alphabetical order, without reordering it."
related : [alphabetize-siblings]
layout  : doc
---

# unsorted-positionals

<RuleLayout rule="unsorted_positionals">

`unsorted-positionals` reports a run of positionally-bound names that sits out of alphabetical order and leaves the reorder to a hand that can check the callers. Alphabetical order gives a reader the same landmarks in a positional run that [[alphabetize-siblings]] gives everywhere else, yet *Prose* never reorders the run, because each name's slot is part of the call contract. Every positional call binds by slot, and a single-file formatter cannot see the callers in other modules, in frameworks, or behind dynamic dispatch, so moving a name would silently rebind them.

Two constructs carry such a run, and the first is a function's positional-or-keyword parameters, free function and method alike, because a method's callers bind by slot exactly as a free function's do. The second is the annotated field run of a class whose header generates a positional constructor, such as a `NamedTuple` base or a `@dataclass` decorator among the headers [[alphabetize-siblings]] lists as pinning the field run. The fields become that constructor's parameters, so a call like `Window(1920, 1080)` binds them in source order.

A function whose decorator is a call carrying positional arguments (*`pytest.mark.parametrize(...)`, `click.argument(...)`, and the like*) draws no report, because the decorator may bind values to the parameters by slot. The field run of a class whose header carries `# prose: keep` draws no report either, because the marker already states that the order written is deliberate. The `self` and `cls` receivers, the positional-only parameters before the `/`, a `ClassVar` declaration, and the `dataclasses.KW_ONLY` sentinel each drop from the run rather than silencing it.

The target order puts the required names in alphabetical order ahead of the defaulted names in alphabetical order, rather than plain alphabetical order throughout, because Python permits nothing else. A required field after a defaulted one raises `TypeError: non-default argument 'zebra' follows default argument 'alpha'` the moment the dataclass is created.

</RuleLayout>
