---
layout: page
title: Home
sidebar: false
aside: false
---

<script setup lang="ts">
import LandingCta          from './.vitepress/theme/components/landing/Cta.vue'
import LandingFeatures     from './.vitepress/theme/components/landing/Features.vue'
import LandingHero         from './.vitepress/theme/components/landing/Hero.vue'
import LandingMetaphor     from './.vitepress/theme/components/landing/Metaphor.vue'
import LandingRulesMarquee from './.vitepress/theme/components/landing/RulesMarquee.vue'
import LandingSection      from './.vitepress/theme/components/landing/LandingSection.vue'
import LandingWorkflow     from './.vitepress/theme/components/landing/Workflow.vue'
</script>

<section class="landing-page" aria-label="prose overview">
  <LandingHero />
  <LandingSection variant="proof" kicker="The Proof" heading="One file, before and after." centered>
    <Fixture rule="thematic" case="dataclass_definition" variant="landing" />
  </LandingSection>
  <LandingMetaphor />
  <LandingFeatures />
  <LandingRulesMarquee />
  <LandingWorkflow />
  <LandingCta />
</section>
