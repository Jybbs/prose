---
title: Layout Rules
warmth: cool
---

The layout rules decide the shape a bracketed construct takes once it outgrows a single line, exploding a call, signature, collection, or `from … import …` to one entry per line so each binding reads on its own and a later edit touches a single row. The trigger is a width budget like `code-line-length`, a count cap like `max-args`, or both, so the inline shape gives way to the stacked one the moment it stops being legible.
