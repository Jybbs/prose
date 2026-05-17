<script setup lang="ts">
import Kicker from '../Kicker.vue'

import { data as landing } from '../../../data/landing.data'

const links = landing.cta.links
</script>

<template>
  <div class="landing-cta">
    <Kicker variant="rule">The Page</Kicker>
    <h2>Ready to <em>read</em>?</h2>
    <div class="landing-links">
      <a v-for="link in links" :key="link.href" :href="link.href">{{ link.text }}</a>
    </div>
  </div>
</template>
