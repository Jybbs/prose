<script setup lang="ts">
import Kicker from '../base/Kicker.vue'
</script>

<template>
  <div class="not-found">
    <Kicker class="not-found-kicker">
      <span class="not-found-kicker-mark" aria-hidden="true">&times;</span>
      <span>404 &middot; mistyped</span>
    </Kicker>
    <h1 class="not-found-headline">
      A page
      <span class="not-found-strike">misplaced</span>
      <span class="not-found-fix">mistyped</span>.
    </h1>
    <p class="not-found-body">
      The URL didn't resolve to any surface we keep on file. Step back to the home page and try a different route through the catalog.
    </p>
    <a class="not-found-cta" href="/">
      <span>Back to the home page</span>
      <span class="not-found-cta-arrow" aria-hidden="true">&rarr;</span>
    </a>
  </div>
</template>
