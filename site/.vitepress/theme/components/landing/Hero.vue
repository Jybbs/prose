<script setup lang="ts">
import { computed } from 'vue'

import LinkRow from '../base/LinkRow.vue'

import { data as landing } from '../../../data/landing.data'

const actionItems = computed(() =>
  landing.hero.actions.map(action => ({
    className: `landing-hero-action landing-hero-action-${action.theme}`,
    href     : action.href,
    text     : action.text
  }))
)
</script>

<template>
  <div class="landing-hero">
    <div class="landing-hero-backdrop" aria-hidden="true">
      <div class="landing-hero-orb landing-hero-orb-1" />
      <div class="landing-hero-orb landing-hero-orb-2" />
      <div class="landing-hero-orb landing-hero-orb-3" />
    </div>
    <h1 class="landing-hero-wordmark">
      <img src="/title.svg" alt="Prose" width="640" height="236" />
    </h1>
    <p class="landing-hero-tagline">A Python typesetter for the reader.</p>
    <LinkRow :items="actionItems" variant="landing-hero-actions" />
  </div>
</template>
