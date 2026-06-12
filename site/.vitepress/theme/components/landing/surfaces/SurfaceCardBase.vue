<script setup lang="ts">
import { useElementHover, useElementSize, useMouseInElement } from '@vueuse/core'
import { computed, ref, useTemplateRef }                      from 'vue'

import type { RenderedRule }            from '../../../../data/rules.data'
import { formatFolio }                  from '../../../../lib/shared/numerals'
import { categoryOf, FAMILY_META, type RuleFamily } from '../../../../lib/shared/registries'

const props = defineProps<{
  bodyHtml : string
  family   : RuleFamily
  number   : string
  rules    : readonly RenderedRule[]
}>()

const meta     = computed(() => FAMILY_META[props.family])
const category = computed(() => categoryOf(props.family))
const href     = computed(() => `/rules/${props.family}/`)

const rootRef = useTemplateRef<HTMLElement>('root')

const active = useElementHover(rootRef)

const SPOTLIGHT_FALLBACK_PCT = 50
const SPOTLIGHT_PCT_SCALE    = 100

const { elementX: rx, elementY: ry } = useMouseInElement(rootRef)
const { width: rw, height: rh }      = useElementSize(rootRef)

const spotlightX = computed(() => rw.value ? (rx.value / rw.value) * SPOTLIGHT_PCT_SCALE : SPOTLIGHT_FALLBACK_PCT)
const spotlightY = computed(() => rh.value ? (ry.value / rh.value) * SPOTLIGHT_PCT_SCALE : SPOTLIGHT_FALLBACK_PCT)

const hoveredIdx = ref<number | null>(null)
const activeIdx  = computed(() => hoveredIdx.value ?? 0)
const activeRule = computed(() => props.rules[activeIdx.value])
</script>

<template>
  <div
    ref="root"
    class="surface-card surface-card-tab-index"
    :data-family="family"
    :data-category="category"
    :data-active="active"
    :style="{
      '--spotlight-x' : `${spotlightX}%`,
      '--spotlight-y' : `${spotlightY}%`
    }"
  >
    <a
      class="surface-card-cover-link"
      :href="href"
      :aria-label="`See all ${meta.label.toLowerCase()} rules`"
    />
    <span class="surface-card-number">— {{ number }}</span>
    <span class="surface-card-icon" aria-hidden="true">{{ meta.badge }}</span>
    <h3 class="surface-card-label">{{ meta.label }}</h3>
    <p class="surface-card-blurb" v-html="bodyHtml" />
    <div class="surface-card-chips">
      <div class="tab-index">
        <div class="tab-row">
          <a
            v-for="(rule, idx) in rules"
            :key="rule.slug"
            class="tab"
            :class="{ active: idx === activeIdx }"
            :href="rule.href"
            :aria-label="rule.slug"
            @mouseenter="hoveredIdx = idx"
            @focus="hoveredIdx = idx"
          >
            {{ formatFolio(idx + 1) }}
          </a>
        </div>
        <div class="tab-label" aria-live="polite">
          <Transition name="tab-swap" mode="out-in">
            <a
              :key="activeIdx"
              class="tab-label-link"
              :href="activeRule?.href"
              :aria-label="activeRule?.slug"
            >{{ activeRule?.slug }}</a>
          </Transition>
        </div>
      </div>
    </div>
  </div>
</template>
