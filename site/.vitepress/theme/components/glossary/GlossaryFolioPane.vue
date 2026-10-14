<script setup lang="ts">
import InlineProse from '../base/InlineProse.vue'
import FamilyChip  from '../rules/FamilyChip.vue'

import { useGlossaryFolio } from '../../../lib/composables/use-glossary-folio'
import { railPaint }        from '../../../lib/shared/family-rail'
import { formatFolio }      from '../../../lib/shared/numerals'

const { active, activeIndex, filtered, step } = useGlossaryFolio()
</script>

<template>
  <div
    class="glossary-folio-stage"
    :data-family="active?.primaryFamily"
    :style="active ? { '--rail-paint': railPaint(active.families) } : undefined"
  >
    <article v-if="active" class="glossary-folio-pane panel">
      <header class="glossary-folio-head">
        <p class="glossary-folio-folio kicker">
          <FamilyChip v-for="family in active.families" :key="family" :family="family" :linked="false" />
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
      <div class="glossary-folio-body"><InlineProse :nodes="active.definitionNodes" /></div>
    </article>
  </div>
</template>
