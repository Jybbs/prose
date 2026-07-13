<script setup lang="ts">
import { data as cards } from '../../../lib/integrations/integrations.data'
import InlineProse       from '../base/InlineProse.vue'
</script>

<template>
  <div class="int-card-grid">
    <article v-for="card in cards" :key="card.href" class="int-card">
      <p class="int-kicker">{{ card.tagline }}</p>
      <p class="int-title"><a class="int-title-link" :href="card.href">{{ card.title }}</a></p>
      <p class="int-body"><InlineProse :nodes="card.summaryNodes" /></p>
      <p class="int-foot" aria-hidden="true">Open →</p>
    </article>
  </div>
</template>
