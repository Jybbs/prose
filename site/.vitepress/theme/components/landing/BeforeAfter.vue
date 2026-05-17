<script setup lang="ts">
import { data as fixtures } from '../../../data/fixtures.data'

const entry = fixtures['thematic']?.['dataclass_definition']
if (!entry) {
  throw new Error('Fixture "thematic/dataclass_definition" not found for LandingBeforeAfter')
}
</script>

<template>
  <div class="landing-section landing-proof">
    <div class="landing-section-heading centered">
      <p class="landing-kicker">The Proof</p>
      <h2>One file, before and after.</h2>
    </div>
    <div class="landing-proof-grid">
      <div class="landing-proof-side">
        <div class="landing-proof-label">
          <span class="landing-proof-dot landing-proof-dot-before" />
          <span>Before</span>
        </div>
        <div class="landing-proof-code" v-html="entry.inputHtml" />
      </div>
      <div class="landing-proof-arrow" aria-hidden="true">→</div>
      <div class="landing-proof-side">
        <div class="landing-proof-label">
          <span class="landing-proof-dot landing-proof-dot-after" />
          <span>After</span>
        </div>
        <div class="landing-proof-code" v-html="entry.outputHtml" />
      </div>
    </div>
  </div>
</template>
