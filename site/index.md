---
layout: page
sidebar: false
aside: false
---

<script setup lang="ts">
import { defineAsyncComponent } from 'vue'

import { BuiltOn as LandingBuiltOn }   from './.vitepress/theme/components/landing/BuiltOn'
import { Cta as LandingCta }           from './.vitepress/theme/components/landing/Cta'
import { Hero as LandingHero }         from './.vitepress/theme/components/landing/Hero'
import { Metaphor as LandingMetaphor } from './.vitepress/theme/components/landing/Metaphor'
import { LandingSection }              from './.vitepress/theme/components/landing/LandingSection'
import { Surfaces as LandingSurfaces } from './.vitepress/theme/components/landing/surfaces/Surfaces'
import { Workflow as LandingWorkflow } from './.vitepress/theme/components/landing/Workflow'

const LandingTypingDemo = defineAsyncComponent(() =>
  import('./.vitepress/theme/components/landing/TypingDemo').then(module => module.TypingDemo)
)
</script>

<main class="landing-page" aria-label="prose overview">
  <LandingHero />
  <LandingSection kicker="The Proof" heading="From <strong>cramped</strong> to <em>composed</em>." centered>
    <LandingTypingDemo />
  </LandingSection>
  <LandingMetaphor />
  <LandingSurfaces />
  <LandingWorkflow />
  <LandingBuiltOn />
  <LandingCta />
</main>
