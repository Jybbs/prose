<script setup lang="ts">
import { data as landing } from '../../../data/landing.data'

const steps = landing.workflow
</script>

<template>
  <div class="landing-section landing-quickstart">
    <SectionHeading kicker="The Workflow" heading="Four lines from install to running." centered />
    <div class="landing-workflow-grid">
      <div v-for="step in steps" :key="step.number" class="workflow-step">
        <span class="step-number">— {{ step.number }}</span>
        <h3>{{ step.title }}</h3>
        <p v-html="step.bodyHtml" />
        <pre><code>{{ step.code }}</code></pre>
      </div>
    </div>
  </div>
</template>
