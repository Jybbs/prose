<script setup lang="ts">
import Kicker  from '../base/Kicker.vue'
import LinkRow from '../base/LinkRow.vue'

import { data as landing } from '../../../data/landing.data'

const links = landing.cta.links
</script>

<template>
  <div class="landing-cta">
    <Kicker variant="rule">The Page</Kicker>
    <h2>Ready to <em>read</em>?</h2>
    <LinkRow :items="links" variant="landing-links" />
  </div>
</template>
