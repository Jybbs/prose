---
layout: page
title: Home
sidebar: false
aside: false
---

<script setup lang="ts">
import { defineAsyncComponent } from 'vue'

import LandingBuiltOn  from './.vitepress/theme/components/landing/BuiltOn.vue'
import LandingCta      from './.vitepress/theme/components/landing/Cta.vue'
import LandingHero     from './.vitepress/theme/components/landing/Hero.vue'
import LandingMetaphor from './.vitepress/theme/components/landing/Metaphor.vue'
import LandingSection  from './.vitepress/theme/components/landing/LandingSection.vue'
import LandingSurfaces from './.vitepress/theme/components/landing/surfaces/Surfaces.vue'
import LandingWorkflow from './.vitepress/theme/components/landing/Workflow.vue'

const LandingTypingDemo = defineAsyncComponent(() =>
  import('./.vitepress/theme/components/landing/TypingDemo.vue')
)
</script>

<section class="landing-page" aria-label="prose overview">
  <LandingHero />
  <LandingSection variant="proof" kicker="The Proof" heading="From <strong>cramped</strong> to <em>composed</em>." centered>
    <LandingTypingDemo />
  </LandingSection>
  <LandingMetaphor />
  <LandingSurfaces />
  <LandingWorkflow />
  <LandingBuiltOn />
  <LandingCta />
</section>
