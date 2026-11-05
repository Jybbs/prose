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
        class="landing-workflow-section relative grid grid-cols-72-1fr gap-24 border-b-1 border-dashed border-b-workflow-accent-24-divider px-28 py-14 before:absolute before:inset-y-14 before:w-1 before:bg-workflow-accent-28-divider last:border-b-0 max-card-stack:grid-cols-64-1fr max-card-stack:gap-18 max-card-stack:px-22 max-card-stack:pt-22 max-card-stack:pb-24 max-card-stack:before:top-22 max-card-stack:before:bottom-24"
      >
        <aside class="sticky top-96 self-start pe-14 text-right max-card-stack:static" aria-hidden="true">
          <span class="inline-block font-display text-1-8rem leading-none font-500 text-celadon-soft-45 italic max-card-stack:text-1-6rem">{{ chapter.roman }}.</span>
        </aside>
        <div class="grid min-w-0 grid-cols-auto-minmax-1fr items-center gap-x-24 gap-y-14 max-card-stack:grid-cols-1fr max-card-stack:gap-x-0">
          <h3 class="landing-workflow-title col-1 row-start-1 row-end-3 rotate-180 self-center border-none text-center font-display text-1-4rem leading-none font-500 tracking-meta text-celadon-soft-45 italic max-card-stack:row-1 max-card-stack:rotate-none max-card-stack:text-left max-card-stack:text-1-3rem max-card-stack:tracking-0 max-card-stack:text-text-1">{{ chapter.title }}</h3>
          <p class="landing-workflow-prose col-2 row-1 max-w-64ch text-md leading-1-6 max-card-stack:col-1 max-card-stack:row-2"><InlineProse :nodes="chapter.bodyNodes" /></p>
          <div class="landing-workflow-code col-2 row-2 min-w-0 self-start max-card-stack:col-1 max-card-stack:row-3" v-html="chapter.codeHtml" />
        </div>
      </section>
    </article>
  </LandingSection>
</template>
