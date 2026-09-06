---
caption : "Breaks a method chain to one link per line once it overflows `code-line-length` or carries more links than `max-links`, hanging each link beneath the head's dot."
related : [align-equals, reflow-calls, reflow-collections, line-overflow]
layout  : doc
---

# stack-method-chains

<RuleLayout rule="stack_method_chains">

`stack-method-chains` breaks a dotted method chain inside a parenthesis pair and hangs every link beneath the head's own dot, so the chain reads down one aligned column of dots.

Two triggers open the break, one reading the link count and one the width. The count trigger fires on a chain carrying more links than `max-links`, so a chain that fits the width still breaks once it carries enough stages to read as a pipeline. The width trigger fires where the joined single-line form crosses `code-line-length` from the column it sits at, which reaches a two-link chain the count cap leaves alone. A link is a `.name(...)` call, so a long dotted prefix ahead of a single call carries one link and stays where it sits, and a `.name` access that is not itself called shares the row of the link below it.

The head keeps the receiver together with its first call, because a bare receiver alone on a line carries no information, and each link below hangs at the receiver's own width past the head's indent. Where that width would put the dot column further out than `max-shift` allows, the chain takes the full split instead, standing the receiver alone and running every link flush beneath it at one indent. The cap is the same `max-shift` the alignment rules read.

The break only ever opens a chain and never rejoins one, so a chain already hung at its dots keeps that layout even where its joined form would fit, because a count trigger paired with a fit test would alternate forever, the count breaking the chain and the fit test rejoining it on the next pass.

The chain reuses a parenthesis pair the source already carries and settles in the same run that first opens it, leaving each link's argument list to [[reflow-calls]] and the collection inside it to [[reflow-collections]]. Both the count and the width are read against that settled form, so a hand-wrapped link is measured at the width [[reflow-calls]] closes it to. A chain inside a link's argument or inside the receiver is measured from the column the break puts it at and, where it trips a trigger there, breaks in the same text, placed at the indent of the row it ends up on.

A chain spanning a comment keeps its source layout, because breaking it would move the links away from the row the comment describes. A link whose break never closes keeps the chain as written, which covers an argument list already written one argument per line, one past `max-args`, and a multi-line string.

A chain inside an f-string or t-string replacement field is left as written whatever its width, because a line break inside one is PEP 701 syntax that fails to parse before Python 3.12, so an over-wide interpolation is left for [[line-overflow]] to report.

<template #configuration>

<RuleConfigTable />

A three-link chain breaks at the default cap of <ConfigDefault rule="stack-method-chains" facet="max-links" /> even where it fits the line. Setting `max-links = false` leaves `code-line-length` as the only trigger, and setting `max-shift = 0` takes the full split for every chain.

</template>

</RuleLayout>
