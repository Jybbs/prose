---
description: "Draws one icon for each emoji the docs site uses as a badge or a label of its own, in house styles to choose a direction from."
---

<script setup lang="ts">
import './icons.css'

import IconSpecimen from './IconSpecimen.vue'
import NibSpecimen  from './NibSpecimen.vue'
</script>

# Icon Family

The docs site uses some emoji as badges and labels of its own, meaning the badge of each rule family, the badge of each family the glossary adds, and the label of each wheel platform, and every one of them renders in the reader's system emoji font. This page draws one icon for each of those emoji on a 24-unit grid, reading the emoji from `FAMILY_META` and `GLOSSARY_FAMILY_META` in `registries.ts` and from the labels in `.github/scripts/platforms.toml`. The drawings stay the same in every variation and only the house style changes, except that the last variation sets a simpler drawing for some motifs below 22 pixels. A house style takes one side of each axis:

- A line or a fill
- A stroke weight
- Monochrome Ube or a duotone in the family's color
- Round terminals or terminals cut to a point like the nib of the pen-nib P

Each variation sets the icons in the site's family chip at text size and in a 32-pixel badge tile, then at 96 pixels as a legibility check at link-preview card size. An icon in a chip or a tile sits beside its label and is hidden from assistive technology, whereas an icon at card size stands alone and carries the name of its motif as its label.

## Variation 1: Ube Line

This variation outlines every icon in Ube at 2 units with round caps and joins, the stroke the site's `.glyph` icons already draw, so it shows the family in the style the site's interface icons use today.

<IconSpecimen form="line" terminal="round" tone="mono" :weight="2" />

## Variation 2: Duotone Nib Line

This variation thins the outline to 1.5 units and cuts each free stroke end to a point. Each silhouette fills with a tint of its family's color beneath an outline in that color mixed toward the text color, and a wheel-platform icon, which belongs to no family, takes Ube. In the dark scheme, each hue darker than an OKLCH lightness of 0.72 is raised to it, which lifts the `cli` wand's Ube Night and the `layout` basket's Toronto clear of the background and lightens the `engine` owl's Ube and the `formatting` feather's Heath.

<IconSpecimen form="line" terminal="nib" tone="duotone" :weight="1.5" />

## Variation 3: Ube Nib Fill

This variation fills every silhouette solid in Ube at 2.5 units, reverses each detail out of it, and cuts each free stroke end to a point, which gives the heaviest mark on the page and the nearest to the solid pen-nib P.

<IconSpecimen form="fill" terminal="nib" tone="mono" :weight="2.5" />

## Variation 4: Duotone Fill

This variation fills each silhouette in its family's color with round caps at 2 units, then draws every stroke outside a silhouette in that color mixed toward the text color, taking the same hues as Variation 2 in both schemes, so a reader tells the families apart by color before outline.

<IconSpecimen form="fill" terminal="round" tone="duotone" :weight="2" />

## Variation 5: Broad Nib

This variation is the direction the round picked, and the site now draws every badge in it. Each drawing strokes its paths at 0.62 units five times, each copy offset along the 45-degree diagonal, so a stroke crossing the diagonal widens while one running along it stays thin, the way a broad nib lays down ink. Below 22 pixels, the wand, the newspaper, the owl, the basket, the harp, the penguin, and the apple each take a simpler drawing stroked at 1.8 units, the wand's shaft at 2.5 units and the owl's and the penguin's features set as filled dots, while the ladder, the feather, the yarn, and the window keep theirs. From 22 pixels up, a turbulence filter roughens the copies. Every drawing stands 18 units tall on the grid, no wider than 22 units, centered on it. Each badge inks its family's color mixed 42 percent toward the text color on a 16 percent tint of that color, and a wheel-platform badge takes Ube the same way.

<NibSpecimen />
