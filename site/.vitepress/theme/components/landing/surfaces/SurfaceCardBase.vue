<script setup lang="ts">
import { useElementHover, useElementSize, useMouseInElement } from '@vueuse/core'
import { computed, ref, useTemplateRef }                      from 'vue'

import { provideAriaHidden }  from '../../../../lib/composables/use-aria-hidden'
import type { InlineNode }    from '../../../../lib/markdown/inline-nodes'
import type { RenderedRule }  from '../../../../lib/rules/rules.data'
import { evenRows }           from '../../../../lib/shared/even-rows'
import { formatFolio }        from '../../../../lib/shared/numerals'
import * as registries        from '../../../../lib/shared/registries'
import InlineProse            from '../../base/InlineProse.vue'
import RuleTooltipPopper      from '../../rules/RuleTooltipPopper.vue'

const KEY_ROW = { gap: 3, minWidth: 28 }

const SPOTLIGHT_FALLBACK_PCT = 50
const SPOTLIGHT_PCT_SCALE    = 100

const props = defineProps<{
  bodyNodes : InlineNode[]
  duplicate : boolean
  family    : registries.RuleFamily
  number    : string
  rules     : readonly RenderedRule[]
}>()

provideAriaHidden(() => props.duplicate)

const category = computed(() => registries.categoryOf(props.family))
const href     = computed(() => `/rules/${props.family}/`)
const meta     = computed(() => registries.FAMILY_META[props.family])
const tabindex = computed(() => (props.duplicate ? -1 : undefined))

const chipsRef = useTemplateRef<HTMLElement>('chips')
const rootRef  = useTemplateRef<HTMLElement>('root')

const active = useElementHover(rootRef)

const { elementX: rx, elementY: ry } = useMouseInElement(rootRef)
const { width: rw, height: rh }      = useElementSize(rootRef)
const { width: chipsWidth }          = useElementSize(chipsRef)

const spotlightX = computed(() => rw.value ? (rx.value / rw.value) * SPOTLIGHT_PCT_SCALE : SPOTLIGHT_FALLBACK_PCT)
const spotlightY = computed(() => rh.value ? (ry.value / rh.value) * SPOTLIGHT_PCT_SCALE : SPOTLIGHT_FALLBACK_PCT)

const hoveredIdx = ref<number | null>(null)
const activeIdx  = computed(() => hoveredIdx.value ?? 0)
const activeRule = computed(() => props.rules[activeIdx.value])

const keyRows = computed(() => evenRows(
  props.rules.map((rule, idx) => ({ idx, rule })),
  { ...KEY_ROW, available: chipsWidth.value }
))
</script>

<template>
  <div
    ref="root"
    class="surface-card panel"
    :aria-hidden="duplicate || undefined"
    :data-family="family"
    :data-category="category"
    :data-active="active"
    :style="{
      '--key-gap'     : `${KEY_ROW.gap}px`,
      '--spotlight-x' : `${spotlightX}%`,
      '--spotlight-y' : `${spotlightY}%`
    }"
  >
    <a
      class="surface-card-cover-link"
      :href="href"
      :aria-label="`See all ${meta.label.toLowerCase()} rules`"
      :tabindex="tabindex"
    />
    <span class="surface-card-number">— {{ number }}</span>
    <span class="surface-card-icon" aria-hidden="true">{{ meta.badge }}</span>
    <h3 class="surface-card-label">{{ meta.label }}</h3>
    <p class="surface-card-blurb"><InlineProse :nodes="bodyNodes" /></p>
    <div ref="chips" class="surface-card-chips">
      <div class="surface-keys">
        <div v-for="(row, line) in keyRows" :key="line" class="surface-key-row">
          <RuleTooltipPopper v-for="entry in row" :key="entry.rule.slug" :rule="entry.rule">
            <a
              class="surface-key"
              :class="{ active: entry.idx === activeIdx }"
              :href="entry.rule.href"
              :aria-label="entry.rule.slug"
              :tabindex="tabindex"
              @mouseenter="hoveredIdx = entry.idx"
              @focus="hoveredIdx = entry.idx"
            ><span class="folio">{{ formatFolio(entry.idx + 1) }}</span></a>
          </RuleTooltipPopper>
        </div>
        <div class="surface-key-label" aria-live="polite">
          <Transition name="key-strike" mode="out-in">
            <a
              :key="activeIdx"
              class="surface-key-label-link"
              :href="activeRule?.href"
              :aria-label="activeRule?.slug"
              :tabindex="tabindex"
            >{{ activeRule?.slug }}</a>
          </Transition>
        </div>
      </div>
    </div>
  </div>
</template>
