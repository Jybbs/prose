---
caption : "Surfaces a docstring type group the signature or the class body already annotates."
related : [signature-annotations, docstring-wrap, alphabetize]
layout  : doc
---

# restated-types

<RuleLayout rule="restated_types">

A `name (type): description` entry restates in prose what an annotated signature states in code, and only one of the two is checked. A type checker reads the annotation on every run whereas nothing reads the docstring type, so the written copy can stay wrong for the life of the function. The renderers that surface a docstring surface the signature beside it anyway, an editor hover and `help()` both printing the parameter list above the body. `restated-types` reports the parenthesized group, anchoring the diagnostic on the type rather than on the whole entry, and leaves the description it introduces untouched.

An entry resolves against the definition whose body its docstring opens. A parameter-documenting section reads the enclosing function's parameters, the `*args` and `**kwargs` variadics included, since an entry name drops its star prefix before it resolves. An `Attributes:` section reads the class body's annotated fields. Google style spells the parameter heading several ways, so `Args:`, `Arguments:`, `Parameters:`, `Keyword Args:`, `Keyword Arguments:`, `Other Args:`, `Other Arguments:`, `Other Params:`, and `Other Parameters:` all document parameters alike.

The report holds back wherever the docstring is carrying its own weight. A parameter with no annotation leaves the docstring as the only place its type is written, which is the gap [[signature-annotations]] pushes into the code instead. An entry naming no member of the set its section documents resolves against nothing, so a `Returns:` or `Raises:` entry that happens to share a parameter's name stays silent. A module docstring documents no signature and no class body, leaving every entry inside it unresolved.

Nothing here is rewritten, because deciding which of two disagreeing types is correct needs a reader rather than a formatter.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[restated-types]` directive.

</template>

</RuleLayout>
