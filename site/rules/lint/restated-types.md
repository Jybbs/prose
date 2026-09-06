---
caption : "Reports the parenthesized type in a docstring entry that the signature or the class body already annotates."
related : [signature-annotations, wrap-docstrings, alphabetize-siblings]
layout  : doc
---

# restated-types

<RuleLayout rule="restated_types">

`restated-types` reports the parenthesized type in a `name (type): description` docstring entry when the signature or the class body already annotates that member, anchoring the diagnostic on the type group rather than the whole entry and leaving the description it introduces as written. The docstring copy restates in prose what the annotation states in code, and only the annotation is checked, because a type checker reads it on every run whereas nothing reads the docstring type. The written copy can stay wrong for the life of the function as a result, and the tools that render a docstring render the signature beside it anyway, an editor hover and `help()` both printing the parameter list above the body.

An entry resolves against the definition whose body its docstring opens. A parameter-documenting section reads the enclosing function's parameters, the `*args` and `**kwargs` variadics included, because an entry name drops its star prefix before it resolves. An `Attributes:` section reads the class body's annotated fields. Google style spells the parameter heading several ways, so `Args:`, `Arguments:`, `Parameters:`, `Keyword Args:`, `Keyword Arguments:`, `Other Args:`, `Other Arguments:`, `Other Params:`, and `Other Parameters:` all document parameters alike.

No report is made where the docstring is the only place a type is written. A parameter with no annotation leaves the docstring type as the sole copy, which is the gap [[signature-annotations]] reports in the code instead. An entry naming no member of the set its section documents resolves against nothing, so a `Returns:` or `Raises:` entry that shares a parameter's name stays silent. A module docstring documents no signature and no class body, so every entry inside it stays unresolved.

Nothing here is rewritten, because deciding which of two disagreeing types is correct takes a reader rather than a formatter.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[restated-types]` directive.

</template>

</RuleLayout>
