<script setup lang="ts">
import { useGlossaryFolio } from './use-glossary-folio'
import { formatFolio }      from '../../../lib/shared/numerals'

const { active, activeIndex, filtered, step } = useGlossaryFolio()
</script>

<template>
  <div class="glossary-folio-stage" :data-family="active?.family">
    <article v-if="active" class="glossary-folio-pane">
      <header class="glossary-folio-head">
        <p class="glossary-folio-folio">
          <span class="glossary-folio-chip">
            <span class="glossary-folio-badge" aria-hidden="true">{{ active.familyBadge }}</span>
            <span class="glossary-folio-folio-family">{{ active.familyLabel }}</span>
          </span>
          <span class="glossary-folio-folio-sep" aria-hidden="true">·</span>
          <span>{{ activeIndex < 0 ? '–' : formatFolio(activeIndex + 1) }} / {{ formatFolio(filtered.length) }}</span>
        </p>
        <div class="glossary-folio-steps">
          <button type="button" class="glossary-folio-step" aria-label="Previous entry" @click="step(-1)">
            <span aria-hidden="true">‹</span>
          </button>
          <button type="button" class="glossary-folio-step" aria-label="Next entry" @click="step(1)">
            <span aria-hidden="true">›</span>
          </button>
        </div>
      </header>
      <h2 class="glossary-folio-headword">
        <a v-if="active.href" :href="active.href">{{ active.slug }}</a>
        <span v-else>{{ active.slug }}</span>
      </h2>
      <p v-if="active.aliases.length > 0" class="glossary-folio-aliases">
        <span class="glossary-folio-aliases-label">also</span>
        <code v-for="alias in active.aliases" :key="alias" class="glossary-folio-aliases-chip">{{ alias }}</code>
      </p>
      <div class="glossary-folio-body" v-html="active.definitionHtml" />
    </article>
  </div>
</template>
