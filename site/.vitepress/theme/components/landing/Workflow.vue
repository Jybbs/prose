<script setup lang="ts">
import { romanize } from 'romans'
import { computed } from 'vue'

import LandingSection from './LandingSection.vue'

import { data as landing } from '../../../lib/landing/landing.data'
import InlineProse         from '../base/InlineProse.vue'

const chapters = computed(() =>
  landing.workflow.map((s, i) => ({ ...s, roman: romanize(i + 1) }))
)
</script>

<template>
  <LandingSection
    centered
    heading="<em>Open</em> with these few lines."
    kicker="The Workflow"
  >
    <article class="landing-workflow mt-24 flex flex-col panel bg-bg-soft">
      <section
        v-for="chapter in chapters"
        :key="chapter.number"
        class="landing-workflow-section relative grid grid-cols-72-1fr gap-24 px-28 py-14 before:absolute before:inset-y-14 before:w-1 before:bg-workflow-accent-28-divider card-stack:grid-cols-64-1fr card-stack:gap-18 card-stack:px-22 card-stack:pt-22 card-stack:pb-24 card-stack:before:top-22 card-stack:before:bottom-24"
      >
        <aside class="landing-workflow-gutter sticky top-96 pe-14 text-right card-stack:static" aria-hidden="true">
          <span class="inline-block font-display text-1-8rem leading-1 font-500 text-workflow-accent-70-text-1 italic card-stack:text-1-6rem">{{ chapter.roman }}.</span>
        </aside>
        <div class="grid min-w-0 grid-cols-auto-minmax-1fr items-center gap-x-24 gap-y-14 card-stack:grid-cols-1fr card-stack:gap-x-0">
          <h3 class="landing-workflow-title col-1 row-start-1 row-end-3 self-center border-none text-center font-display text-1-4rem leading-1 font-500 text-workflow-accent-70-text-1 italic card-stack:row-1 card-stack:text-left card-stack:text-1-3rem card-stack:text-text-1">{{ chapter.title }}</h3>
          <p class="landing-workflow-prose col-2 row-1 max-w-64ch text-md leading-1-6 card-stack:col-1 card-stack:row-2"><InlineProse :nodes="chapter.bodyNodes" /></p>
          <div class="landing-workflow-code col-2 row-2 min-w-0 card-stack:col-1 card-stack:row-3" v-html="chapter.codeHtml" />
        </div>
      </section>
    </article>
  </LandingSection>
</template>
