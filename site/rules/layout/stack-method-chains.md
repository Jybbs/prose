---
caption : "Breaks a fluent method chain across lines once it overflows the width or carries more links than the cap, hanging each link beneath the head's dot."
related : [align-equals, reflow-calls, reflow-collections, line-overflow]
layout  : doc
---

# stack-method-chains

<RuleLayout rule="stack_method_chains">

`stack-method-chains` breaks a dotted method chain inside a parenthesis pair and hangs every link beneath the head's own dot, giving the chain one aligned column to read down.

Two triggers open the break. The count trigger fires on a chain carrying more links than `max-links`, so a chain that fits the width still breaks once it carries enough stages to read as a pipeline. The length trigger fires where the joined single-line form crosses `code-line-length` from the column it lands at, which reaches a two-link chain the count cap leaves alone. A link is a `.name(...)` call, so a long dotted prefix ahead of a single call carries one link and stays where it sits, and a `.name` access that is not itself called shares the row of the link below it.

The head holds the receiver together with its first call, a bare receiver alone on a line carrying no information, and each link below hangs at the receiver's own width past the head's indent. Where that width would shift the dot column further than `max-shift` allows, the chain takes the full split instead, standing the receiver alone and running every link flush beneath it. The cap is the same `max-shift` the alignment rules read.

The break only ever opens a chain, never rejoining one, so a chain already hung at its dots holds that shape even where its joined form would fit. A count trigger opposed by a fit test would alternate forever, the count breaking the chain and the fit test rejoining it on the pass after.

The chain reuses a parenthesis pair the source already carries and settles in the run that first opens it, leaving each link's argument list to [[reflow-calls]] and the collection inside it to [[reflow-collections]]. Both the count and the width read against that settled form, so a hand-wrapped link is measured at the width [[reflow-calls]] closes it to.

A chain spanning a comment holds its source shape, relocating the links carrying the comment away from the row it annotates. A link holds open on a break that never closes, covering the flush column, an argument list past `max-args`, and a multi-line string.

A chain inside an f-string or t-string replacement field is opaque to layout whatever its width, a spliced line break there being PEP 701 syntax that fails before Python 3.12, leaving an over-wide interpolation for [[line-overflow]] to report.

<template #configuration>

<RuleConfigTable />

A three-link chain breaks at the default cap of `2` even where it fits the line. Setting `max-links = false` leaves `code-line-length` as the only trigger, and setting `max-shift = 0` takes the full split for every chain.

</template>

</RuleLayout>
