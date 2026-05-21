---
layout: page
title: The Page iterations
sidebar: false
aside: false
head:
  - - meta
    - name: robots
      content: noindex
---

<script setup lang="ts">
import PageCompressedSpine from '../../.vitepress/theme/components/iterations/the-page/PageCompressedSpine.vue'
import PageInlineMarquee   from '../../.vitepress/theme/components/iterations/the-page/PageInlineMarquee.vue'
import PageFolioBottom     from '../../.vitepress/theme/components/iterations/the-page/PageFolioBottom.vue'
import PageRectoVerso      from '../../.vitepress/theme/components/iterations/the-page/PageRectoVerso.vue'
</script>

<div class="iter-page">

<header class="iter-head">
<p class="iter-kicker">— The Page · Cta refinements —</p>
<h1>The Page · install &amp; release-history variants</h1>
<p class="iter-lede">Four compact-but-rich treatments of the landing Cta. Each variant preserves the install command, PyPI release chronology, and Quick start link, while leaning harder on typographic hierarchy and trimming the vertical signature of the production Release Spine.</p>
</header>

<section class="iter-section">
<div class="iter-label">
<span class="iter-tag">C · Compressed Spine</span>
<h2>Tighter rail with a single-row install &amp; primary</h2>
<p>The production silhouette, compacted. Rail trims to 180px with smaller version glyphs, the lede loses a step in size, and install command plus Quick start collapse onto one row. Same anatomy as Release Spine, half the breathing room.</p>
</div>
<PageCompressedSpine />
</section>

<section class="iter-section">
<div class="iter-label">
<span class="iter-tag">C¹ · Inline Marquee</span>
<h2>One ruled bar of kicker, install, and primary</h2>
<p>Kicker, install command, and Quick start ride a single top rule as inline marks. The lede sits below in display face, and the release chronology runs underneath as a horizontal marquee strip with version glyphs as columns rather than a side rail.</p>
</div>
<PageInlineMarquee />
</section>

<section class="iter-section">
<div class="iter-label">
<span class="iter-tag">C² · Folio Bottom</span>
<h2>Typeset heading above a numbered colophon</h2>
<p>The lede ascends to a typeset heading with the install command and Quick start beside it on a single line. Below a thin rule, every PyPI release falls into a two-column numbered colophon with dotted leaders connecting version to date.</p>
</div>
<PageFolioBottom />
</section>

<section class="iter-section">
<div class="iter-label">
<span class="iter-tag">C³ · Recto-Verso</span>
<h2>Two-page spread with the lede running across the gutter</h2>
<p>The tagline runs full-width across the top as a single hairline-ruled slug. Beneath, a recto-verso spread anchors the install command and Quick start on the left page, with the release chronology stacked to the right of a center gutter rule.</p>
</div>
<PageRectoVerso />
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
