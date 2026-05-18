<script setup lang="ts">
import LandingSection from './LandingSection.vue'

import { data as landing } from '../../../data/landing.data'

const steps = landing.workflow
</script>

<template>
  <LandingSection
    centered
    heading="Four lines from install to running."
    kicker="The Workflow"
    variant="quickstart"
  >
    <div class="landing-workflow-grid">
      <div v-for="step in steps" :key="step.number" class="workflow-step">
        <span class="step-number">— {{ step.number }}</span>
        <h3>{{ step.title }}</h3>
        <p v-html="step.bodyHtml" />
        <pre><code>{{ step.code }}</code></pre>
      </div>
    </div>
  </LandingSection>
</template>
