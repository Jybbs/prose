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
import SectionHeading      from './.vitepress/theme/components/landing/SectionHeading.vue'
import LandingWorkflow     from './.vitepress/theme/components/landing/Workflow.vue'
</script>

<section class="landing-page" aria-label="prose overview">
  <LandingHero />
  <section class="landing-section landing-proof">
    <SectionHeading kicker="The Proof" heading="One file, before and after." centered />
    <Fixture rule="thematic" case="dataclass_definition" variant="landing" />
  </section>
  <LandingMetaphor />
  <LandingFeatures />
  <LandingRulesMarquee />
  <LandingWorkflow />
  <LandingCta />
</section>
