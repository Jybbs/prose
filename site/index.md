---
layout: page
title: Home
sidebar: false
aside: false
---

<script setup lang="ts">
import LandingBuiltOn    from './.vitepress/theme/components/landing/BuiltOn.vue'
import LandingCta        from './.vitepress/theme/components/landing/Cta.vue'
import LandingHero       from './.vitepress/theme/components/landing/Hero.vue'
import LandingMetaphor   from './.vitepress/theme/components/landing/Metaphor.vue'
import LandingSection    from './.vitepress/theme/components/landing/LandingSection.vue'
import LandingSurfaces   from './.vitepress/theme/components/landing/surfaces/Surfaces.vue'
import LandingTypingDemo from './.vitepress/theme/components/landing/TypingDemo.vue'
import LandingWorkflow   from './.vitepress/theme/components/landing/Workflow.vue'
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
