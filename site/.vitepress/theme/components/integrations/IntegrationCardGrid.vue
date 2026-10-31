<script setup lang="ts">
import { data as cards } from '../../../lib/integrations/integrations.data'
import InlineProse       from '../base/InlineProse.vue'
</script>

<template>
  <div class="my-1-5rem grid grid-cols-fill-260 gap-1rem">
    <article v-for="card in cards" :key="card.href" class="int-card relative rounded-sm border-1 border-divider bg-bg-soft px-1-1rem pt-1rem pb-0-85rem text-text-1 transition-border-color-and-background hover:border-ube hover:bg-bg">
      <p class="mt-0 mb-0-3rem font-mono text-kicker-sm tracking-label-loose text-text-3 uppercase">{{ card.tagline }}</p>
      <p class="mt-0 mb-0-5rem border-none font-display text-1-2rem font-500 text-text-1 italic"><a class="text-inherit no-underline after:absolute after:inset-0" :href="card.href">{{ card.title }}</a></p>
      <p class="int-body mt-0 mb-0-7rem text-0-88rem leading-1-45 text-text-2"><InlineProse :nodes="card.summaryNodes" /></p>
      <p class="m-0 font-mono text-kicker tracking-meta text-ube" aria-hidden="true">Open →</p>
    </article>
  </div>
</template>
