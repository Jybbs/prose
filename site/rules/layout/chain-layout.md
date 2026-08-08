---
caption : "Breaks a fluent method chain across lines once it overflows the width or carries more links than the cap, hanging each link beneath the head's dot."
related : [align-equals, call-layout, collection-layout, line-overflow]
layout  : doc
---

# chain-layout

<RuleLayout rule="chain_layout">

A dotted method chain packed onto one line reads as **a single run of punctuation**, and the eye has to parse the dots to find where one stage ends and the next begins. The same chain broken one link per line reads as **a pipeline**, each stage a row the reader takes in at a glance. `chain-layout` breaks a chain inside a parenthesis pair and hangs every link beneath the head's own dot, giving the chain one aligned column to scan down. The column is the alignment family's existing primitive reaching a new token, wherein [[align-equals]] builds from the operator and this builds from the `.`.

Two triggers open the break, the count trigger firing on a chain carrying more links than `max-links`, so a chain that fits the width still breaks once it carries enough stages to read as a pipeline. The length trigger fires on a chain whose joined single-line form crosses `code-line-length` from the column it lands at, which reaches a two-link chain the count cap leaves alone. A link is a `.name(...)` call, so a long dotted prefix ahead of a single call carries one link and stays where it sits, and a `.name` access that is not itself called shares the row of the link below it.

The head holds the receiver together with its first call, because a bare receiver alone on a line carries no information, and each link below it hangs at the receiver's own width past the head's indent. Where that width would shift the dot column further than `max-shift` allows, the chain falls back to the full split instead, standing the receiver alone on its line and running every link flush beneath it. The cap is the same `max-shift` the alignment rules read, so one knob governs how far any column may travel.

The break only ever opens a chain, never rejoining one, so a chain already hung at its dots holds that shape even where its joined form would fit the budget. A count trigger opposed by a fit test would alternate forever, wherein the count breaks the chain and the fit test rejoins it on the pass after.

A chain sits inside a parenthesis pair rather than behind backslash continuations, reusing a pair the source already carries so a parenthesized chain gains no second one. The whole chain settles in the run that first opens it, every link placed against the indentation and width that run emits, leaving each link's own argument list to [[call-layout]] and the collection inside it to [[collection-layout]], both of which read the columns this break produces. A chain spanning a comment holds its source shape, since relocating the links would carry the comment away from the row it annotates, and one whose links still span lines once their fractured argument lists close up is left where it sits. Both the count and the width are read against that settled form rather than against the source, so a link the author hand-wrapped is measured and rendered at the width [[call-layout]] closes it to, and a hand-wrapped chain reaches its broken shape in the run that first sees it rather than the one after. What holds a link open is a break that never closes, covering the flush column, an argument list past `max-args`, and a link spanning a multi-line string. A comment anywhere in the chain is caught earlier, by the comment guard above. A chain inside an f-string or t-string replacement field is opaque to layout whatever its width, because a line break spliced into a single-quoted field is PEP 701 syntax that parses on Python 3.12 and later and fails everywhere earlier, leaving an over-wide interpolation for [`line-overflow`](/rules/lint/line-overflow) to report.

<template #configuration>

<RuleConfigTable />

A three-link chain breaks at the default cap of `2` even where it fits the line. Setting `max-links = false` leaves `code-line-length` as the only trigger, and setting `max-shift = 0` takes the full split for every chain.

</template>

</RuleLayout>
