<script setup lang="ts">
import LandingSection from './LandingSection.vue'

import { data as landing } from '../../../data/landing.data'

const features = landing.features
</script>

<template>
  <LandingSection
    centered
    heading="Three rule families. Eighteen rules."
    kicker="The Surfaces"
    variant="menu"
  >
    <template #heading-extra>
      <a href="/rules/" class="landing-small-button">All rules</a>
    </template>
    <div class="landing-feature-grid">
      <a v-for="feature in features" :key="feature.title" :href="feature.link" class="landing-feature-card">
        <span class="card-number">— {{ feature.number }}</span>
        <span class="card-icon">{{ feature.icon }}</span>
        <h3>{{ feature.title }}</h3>
        <p v-html="feature.bodyHtml" />
        <span class="card-link">{{ feature.cta }} →</span>
      </a>
    </div>
  </LandingSection>
</template>
