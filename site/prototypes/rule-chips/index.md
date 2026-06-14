# Rule Chip Treatments

This page holds one corpus constant, a dense inline list of rule references spanning the alignment, ordering, formatting, lint, and docs families in a deliberately clashing order, and varies only how each chip wears its family color. The current treatment tints every chip's text its full family hue, so a list that crosses families reads as *color-vom*, one hue swinging to the next line after line. Each variation below pulls that color back to a quieter signal while keeping the family legible, so the comparison stays purely about the shape of the affordance rather than the corpus it renders.

## Variation 1 — Baseline (Current Treatment)

The current production rendering, reproduced unchanged so the candidates read against the problem they solve. Each chip prints its slug in mono with both the glyphs and a 1px underline carrying the full family color, so the hue saturates every pixel of the word. Across a cross-family corpus the chips swing from one family color to the next line after line, which is the *color-vom* every candidate below sets out to quiet.

<RuleChipsBaseline />

## Variation 2 — Neutral Ink, Family Dot

Every slug holds neutral primary ink in mono, so the run reads as one calm column rather than a quilt of competing hues. Family shrinks to a single six-pixel dot before each slug, drawing its fill from the per-chip `data-family` accent, and the dots line up at the left edge to scan together as a quiet legend. Color blooms only on hover, where the family tint washes faintly into the chip.

<RuleChipsDot />

## Variation 3 — Family Underline

The slug stays neutral mono ink and the family signal drops below the baseline, a thin always-on underline drawn with the same background-gradient idiom the production rule links already use. Because the words never take a hue, adjacent chips read as one calm line and the family color registers as a quiet stripe along the bottom edge. A faint thickening on hover keeps the chip interactive without re-tinting the ink.

<RuleChipsUnderline />

## Variation 4 — Faint Family Wash

Each reference becomes a snug mono pill whose text stays neutral while the family moves entirely into the surface, a low-opacity wash mixed from the family color at twelve percent with a hair-thin colored left border. The slug is never tinted, so five families sit shoulder to shoulder as quiet background fields rather than clashing foreground hues. Hover lifts the wash so a chip can be picked out without raising the resting volume.

<RuleChipsWash />

## Variation 5 — Single Brand Accent

Per-family color leaves the resting state entirely, so every chip renders the same neutral mono text under a single uniform Ube underline and the list scans as one color top to bottom. Family is held back until the reader reaches a chip, where hover and focus-visible shift the underline to the chip's own family color and wash a faint matching tint behind it. The trade is deliberate, surrendering at-rest family legibility because the corpus is too dense for five hues to coexist quietly.

<RuleChipsAccent />

## Variation 6 — Family Grouped

The flat clashing list restructures into per-family sections, each led by a single colored marker, a small family swatch beside the family name as a mono uppercase kicker, so the hue appears once per family rather than once per chip. The slug chips inside a group all read in neutral mono, leaving the eye to scan monochrome labels under one quiet color cue. Color now signals family structurally through the group head instead of staining every token.

<RuleChipsGrouped />
