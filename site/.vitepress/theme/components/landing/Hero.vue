<script setup lang="ts">
import '../../css/landing.css'

import { data as landing } from '../../../data/landing.data'

const actions = landing.hero.actions
</script>

<template>
  <div class="landing-hero">
    <div class="landing-hero-backdrop" aria-hidden="true">
      <div class="landing-hero-orb landing-hero-orb-1" />
      <div class="landing-hero-orb landing-hero-orb-2" />
      <div class="landing-hero-orb landing-hero-orb-3" />
    </div>
    <img class="landing-hero-logo" src="/logo.svg" alt="prose" width="160" height="160" />
    <h1 class="landing-hero-name">Prose</h1>
    <p class="landing-hero-tagline">A Python typesetter for the reader.</p>
    <div class="landing-hero-actions">
      <a
        v-for="action in actions"
        :key="action.href"
        :href="action.href"
        :class="['landing-hero-action', `landing-hero-action-${action.theme}`]"
      >
        {{ action.text }}
      </a>
    </div>
  </div>
</template>
