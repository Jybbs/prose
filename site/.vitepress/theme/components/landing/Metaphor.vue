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
    <div class="landing-metaphor-body">
      <div class="landing-metaphor-copy">
        <p>
          Code is read far more often than it is written. A reader's eye moves
          down a page and across adjacent lines looking for parallels, patterns,
          and <em>shape</em>. When every <code>=</code> sits at a different
          column and every collection is compressed onto one line, that shape
          disappears, forcing the eye to slow down.
        </p>
        <p>
          <em>Prose</em> restores it. Aligned columns let the eye skim,
          one-per-line collections make each entry a unit, and alphabetized
          groupings give every reader the same landmarks.
        </p>
      </div>
      <div class="landing-definition">
        <div class="definition-word">prose <span>/prōz/</span></div>
        <p>1. written language in its ordinary form, without metrical structure.</p>
        <p>2. an opinionated formatter that lays Python out so a reader can read it.</p>
      </div>
    </div>
  </LandingSection>
</template>
