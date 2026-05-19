<script setup lang="ts">
import LinkRow from '../base/LinkRow.vue'

import { data as landing } from '../../../data/landing.data'

const actionItems = landing.hero.actions.map(action => ({
  className : `landing-hero-action landing-hero-action-${action.theme}`,
  href      : action.href,
  text      : action.text
}))
</script>

<template>
  <div class="landing-hero">
    <h1 class="landing-hero-wordmark">
      <img src="/title.svg" alt="Prose" width="640" height="236" />
    </h1>
    <p class="landing-hero-tagline">A Python typesetter for the reader.</p>
    <LinkRow :items="actionItems" variant="landing-hero-actions" />
  </div>
</template>
