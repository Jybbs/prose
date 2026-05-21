<script setup lang="ts">
import LandingSection from './LandingSection.vue'
</script>

<template>
  <LandingSection
    centered
    heading="Code, read like <em>prose</em>."
    kicker="The Idea"
    variant="metaphor"
  >
    <aside class="landing-definition">
      <p class="landing-definition-kicker">— Reference · prose —</p>
      <header class="landing-definition-head">
        <h3 class="landing-definition-word">prose</h3>
        <span class="landing-definition-pron">/prōz/</span>
        <span class="landing-definition-leader" aria-hidden="true" />
        <span class="landing-definition-pos">n.</span>
      </header>
      <ol class="landing-definition-list">
        <li>
          <span class="landing-definition-num">1</span>
          <span class="landing-definition-body">written language in its ordinary form, without metrical structure.</span>
        </li>
        <li>
          <span class="landing-definition-num">2</span>
          <span class="landing-definition-body">an opinionated formatter that lays Python out so a reader can read it.</span>
        </li>
      </ol>
      <hr class="landing-definition-rule" aria-hidden="true" />
      <div class="landing-definition-copy">
        <p>
          Code is read far more often than it is written. A reader's eye moves
          down a page and across adjacent lines looking for parallels, patterns,
          and <em>shape</em>. When every assignment sits at a different
          column and every collection is compressed onto one line, that shape
          disappears, forcing the eye to slow down.
        </p>
        <p>
          <em>Prose</em> restores it. Aligned columns let the eye skim,
          one-per-line collections make each entry a unit, and alphabetized
          groupings give every reader the same landmarks.
        </p>
      </div>
    </aside>
  </LandingSection>
</template>
