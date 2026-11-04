<script setup lang="ts">
import LandingSection from './LandingSection.vue'
</script>

<template>
  <LandingSection
    centered
    class="border-t-1 border-t-divider pt-72"
    heading="Code, made <em>legible</em>."
    kicker="The Idea"
  >
    <aside class="relative mt-36 panel bg-bg-soft px-40 pt-32 pb-36">
      <p class="kicker mb-16">— a definition —</p>
      <header class="mb-22 flex items-baseline gap-10">
        <h3 class="border-none font-display text-2-2rem leading-none font-500 text-balance text-ube">prose</h3>
        <span class="font-mono text-sm text-text-3">/prōz/</span>
        <span class="mx-6 leader flex-1 opacity-50" aria-hidden="true" />
        <span class="font-display text-1rem text-text-3 italic">n.</span>
      </header>
      <ol class="flex flex-col gap-12">
        <li class="flex items-baseline gap-12">
          <span class="landing-definition-num flex-none rounded-pill bg-brand-1/faint px-8 py-1 font-mono text-xxs text-brand-1">1</span>
          <span class="text-md leading-1-55 text-text-2 italic">written language in its ordinary form, without metrical structure.</span>
        </li>
        <li class="flex items-baseline gap-12">
          <span class="landing-definition-num flex-none rounded-pill bg-brand-1/faint px-8 py-1 font-mono text-xxs text-brand-1">2</span>
          <span class="text-md leading-1-55 text-text-2 italic">an opinionated formatter that lays Python out so a reader can read it.</span>
        </li>
      </ol>
      <hr class="my-22 border-0 border-t-1 border-t-divider" aria-hidden="true" />
      <div>
        <p class="mb-14 text-1rem leading-1-65 last:mb-0">
          Code is read far more often than it is written. A reader's eye moves
          down a page and across adjacent lines looking for parallels, patterns,
          and <em>shape</em>. When every assignment sits at a different
          column and every collection is compressed onto one line, that shape
          disappears, forcing the eye to slow down.
        </p>
        <p class="mb-14 text-1rem leading-1-65 last:mb-0">
          <em>Prose</em> restores it. Aligned columns let the eye skim,
          dictionaries written one entry per line make each entry a unit, and
          alphabetized groupings give every reader the same landmarks.
        </p>
      </div>
    </aside>
  </LandingSection>
</template>
