---
layout: page
title: The Stack iterations
sidebar: false
aside: false
head:
  - - meta
    - name: robots
      content: noindex
---

<script setup lang="ts">
import StackColophon    from '../../.vitepress/theme/components/iterations/the-stack/StackColophon.vue'
import StackMasthead    from '../../.vitepress/theme/components/iterations/the-stack/StackMasthead.vue'
import StackPrinterSlug from '../../.vitepress/theme/components/iterations/the-stack/StackPrinterSlug.vue'
</script>

<div class="iter-page">

<header class="iter-head">
<p class="iter-kicker">— The Stack · Card refinements —</p>
<h1>The Stack · tool credit variants</h1>
<p class="iter-lede">Three distinct treatments of the tool-credit section. The credit list is pruned to load-bearing tools (Rust, Ruff, uv, mise) with pre-commit and VitePress relegated to secondary lines where the design supports them. Each variant explores a different typographic frame.</p>
</header>

<section class="iter-section">
<div class="iter-label">
<span class="iter-tag">B · Colophon</span>
<h2>Typeset colophon list with dotted leaders</h2>
<p>Each tool gets a single row: italic Fraunces name with inline glyph, mono role kicker, dotted leader, and a one-line Lora note describing what <em>Prose</em> uses it for. Reads as a book's printer-colophon page, where the publication credits each line of provenance.</p>
</div>
<StackColophon />
</section>

<section class="iter-section">
<div class="iter-label">
<span class="iter-tag">B¹ · Masthead</span>
<h2>Three-cell masthead with a "set in" footer</h2>
<p>The load-bearing trio (Rust, Ruff, uv) takes a full-width three-cell masthead, each cell carrying its glyph, Fraunces wordmark, mono role kicker, and italic blurb. Below a thin centered rule, a single "and set in" line credits mise, pre-commit, and VitePress in mono with italic role suffixes.</p>
</div>
<StackMasthead />
</section>

<section class="iter-section">
<div class="iter-label">
<span class="iter-tag">B² · Printer's slug</span>
<h2>Four tall columns with circle medallions</h2>
<p>Each load-bearing tool stands as its own tall column: glyph mounted inside a circle medallion (mirroring the family-emoji circle on the rules plate), Fraunces name, mono role, hairline rule, and a short italic note. A mono footer line names the lighter credits (pre-commit, VitePress, GitHub Actions). mise renders without an iconify mark.</p>
</div>
<StackPrinterSlug />
</section>

</div>

<style scoped>
.iter-page {
  max-width : 1280px;
  margin    : 0 auto;
  padding   : 48px 24px 96px;
}

.iter-head {
  margin-bottom : 56px;
}

.iter-kicker {
  margin         : 0 0 12px;
  font-family    : var(--vp-font-family-mono);
  font-size      : var(--prose-kicker-size);
  letter-spacing : var(--prose-kicker-tracking);
  text-transform : uppercase;
  color          : var(--vp-c-text-3);
}

.iter-head h1 {
  margin      : 0 0 16px;
  font-family : var(--prose-font-display);
  font-weight : 500;
  font-size   : clamp(2rem, 3.6vw, 2.8rem);
  line-height : 1.1;
}

.iter-head .iter-lede {
  margin      : 0;
  max-width   : 64ch;
  font-family : var(--vp-font-family-base);
  font-size   : 1.05rem;
  line-height : 1.6;
  color       : var(--vp-c-text-2);
}

.iter-section {
  margin-top  : 56px;
  padding-top : 32px;
  border-top  : 1px solid var(--vp-c-divider);
}

.iter-label {
  margin-bottom : 24px;
}

.iter-tag {
  display        : inline-block;
  margin-bottom  : 8px;
  font-family    : var(--vp-font-family-mono);
  font-size      : var(--prose-kicker-size);
  letter-spacing : var(--prose-kicker-tracking);
  text-transform : uppercase;
  color          : var(--vp-c-brand-1);
}

.iter-label h2 {
  margin      : 0 0 8px;
  font-family : var(--prose-font-display);
  font-weight : 500;
  font-size   : 1.65rem;
  line-height : 1.2;
}

.iter-label p {
  margin      : 0;
  max-width   : 64ch;
  font-family : var(--vp-font-family-base);
  font-size   : var(--prose-text-md);
  line-height : 1.6;
  color       : var(--vp-c-text-2);
}
</style>
