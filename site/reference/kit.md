---
description: "Lists every control, chip, and drawing helper the docs site builds from, each rendered live beside its source."
---

# Kit

Every component on this site reaches its shared controls, chips, and drawing helpers through one entry, `site/.vitepress/kit`, and a lint rule fails on an import that goes around it. The sections below render each piece the entry exports, live and in the site's own style, beside its source.

The site suite fails on an export that carries no declaration, so a piece reaches this page in the change that adds it. A declaration holds the piece's use and its examples, in a `.catalog.tsx` file beside the piece.

<KitCatalog />
