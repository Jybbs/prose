<script setup lang="ts">
import Kicker from '../base/Kicker.vue'
</script>

<template>
  <div class="mx-auto my-96 flex max-w-640 flex-col items-start gap-22 px-24">
    <Kicker class="inline-flex items-center gap-10" variant="wide">
      <span class="text-apricot" aria-hidden="true">&times;</span>
      <span>404 &middot; mistyped</span>
    </Kicker>
    <h1 class="not-found-headline max-w-18ch border-none font-display leading-1-08 font-500 -tracking-0-01em text-text-1 italic">
      A page
      <span class="relative mr-8 text-text-3 not-italic line-through decoration-apricot/75 decoration-2">misplaced</span>
      <span class="text-ube">mistyped</span>.
    </h1>
    <p class="max-w-48ch text-1rem leading-1-6 text-text-2">
      The URL didn't resolve to any surface we keep on file. Step back to the home page and try a different route through the catalog.
    </p>
    <a class="not-found-cta mt-6 inline-flex items-center gap-10 rounded-sm border-1 border-apricot bg-bg px-20 py-11 font-display text-1-02rem font-500 text-apricot-70-text-1 italic transition-background-and-color hover:bg-apricot-medium-bg hover:text-text-1" href="/">
      <span>Back to the home page</span>
      <span class="not-found-cta-arrow" aria-hidden="true">&rarr;</span>
    </a>
  </div>
</template>
