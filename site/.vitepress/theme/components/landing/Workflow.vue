<script setup lang="ts">
import { computed } from 'vue'

import LandingSection from './LandingSection.vue'

import { data as landing } from '../../../data/landing.data'
import { toRoman }         from '../../../lib/shared/numerals'

const chapters = computed(() =>
  landing.workflow.map((s, i) => ({ ...s, roman: toRoman(i + 1) }))
)
</script>

<template>
  <LandingSection
    centered
    heading="<em>Open</em> with these few lines."
    kicker="The Workflow"
    variant="quickstart"
  >
    <article class="landing-workflow">
      <section
        v-for="(chapter, idx) in chapters"
        :key="chapter.number"
        class="landing-workflow-section"
        :class="{ 'landing-workflow-section-last': idx === chapters.length - 1 }"
      >
        <aside class="landing-workflow-gutter" aria-hidden="true">
          <span class="landing-workflow-roman">{{ chapter.roman }}.</span>
        </aside>
        <div class="landing-workflow-body">
          <h3 class="landing-workflow-title">{{ chapter.title }}</h3>
          <p class="landing-workflow-prose" v-html="chapter.bodyHtml" />
          <div class="landing-workflow-code" v-html="chapter.codeHtml" />
        </div>
      </section>
    </article>
  </LandingSection>
</template>
