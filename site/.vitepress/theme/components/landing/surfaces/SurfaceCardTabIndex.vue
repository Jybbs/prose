<script setup lang="ts">
import { computed, ref }     from 'vue'

import type { RenderedRule } from '../../../../data/rules.data'
import type { RuleFamily }   from '../../../../lib/shared/registries'

import SurfaceCardBase       from './SurfaceCardBase.vue'

const props = defineProps<{
  bodyHtml : string
  family   : RuleFamily
  icon     : string
  number   : string
  rules    : readonly RenderedRule[]
}>()

const hoveredIdx = ref<number | null>(null)
const activeIdx  = computed(() => hoveredIdx.value ?? 0)
const activeRule = computed(() => props.rules[activeIdx.value])
</script>

<template>
  <SurfaceCardBase
    class="surface-card-tab-index"
    :body-html="bodyHtml"
    :family="family"
    :icon="icon"
    :number="number"
    :rules="rules"
  >
    <template #default="{ rules: items }">
      <div class="tab-index">
        <div class="tab-row">
          <a
            v-for="(rule, idx) in items"
            :key="rule.slug"
            class="tab"
            :class="{ active: idx === activeIdx }"
            :href="`/rules/${rule.slug}`"
            :aria-label="rule.slug"
            @mouseenter="hoveredIdx = idx"
            @focus="hoveredIdx = idx"
          >
            {{ String(idx + 1).padStart(2, '0') }}
          </a>
        </div>
        <div class="tab-label" aria-live="polite">
          <Transition name="tab-swap" mode="out-in">
            <a
              :key="activeIdx"
              class="tab-label-link"
              :href="`/rules/${activeRule?.slug}`"
              :aria-label="activeRule?.slug"
            >{{ activeRule?.slug }}</a>
          </Transition>
        </div>
      </div>
    </template>
  </SurfaceCardBase>
</template>
