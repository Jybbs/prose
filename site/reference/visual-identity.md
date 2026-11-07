---
description: "Sets out the palette, the color roles, the typefaces, the marks, and the family badges the site draws with, each read from its declaration."
---

# Visual Identity

Every color, face, mark, and badge the site draws with is declared once, in the palette module, the stylesheets, and the files under `site/public`. Each specimen below reads its values from those declarations as the site builds or as the page loads, so a change to a declaration reaches this page in the same change.

## Palette

The colors `palette.ts` declares sit in one band, grouped by the declaration that assigns each one. Choosing a strip names the color beneath the band beside its hex, token, and OKLCH value, its contrast against the page ground of each scheme, and every place the site reads it.

<IdentityPalette />

## Roles

A page draws with a small set of color roles, each a custom property the stylesheets read. The passage below sets each role in the scheme its switch names, and pointing at a role in the legend rings every place the passage uses it.

<IdentityRoles />

## Typefaces

Each typographic role sits on the small page below in the site's own styles. The index groups the roles by the face each one computes, and the pane gives the chosen role's face, weight, size, style, and tracking, the tokens its declarations read, and a line of the face's numerals and reference marks.

<IdentityTypefaces />

## Marks

The pen-nib P, the letters, and the tagline compose the lockup. The switches turn each part on or off, the line beneath names the file that draws the parts turned on, and the composed mark sits in a copy of the nav title, where the corner handle or the ruler resizes it. The diamond on the ruler marks the height the nav bar serves the mark at.

<IdentityMarks />

## Family Badges

Every rule family and each cross-cutting concern takes a badge drawn with a broad nib. Choosing a family draws its badge large, stroke by stroke, beside the ink it takes in each scheme, its stroke count, and the number of rules the family holds.

<IdentityBadges />
