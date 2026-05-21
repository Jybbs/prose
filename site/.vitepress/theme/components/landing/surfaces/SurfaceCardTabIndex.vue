<script setup lang="ts">
import { computed, ref }     from 'vue'

import type { RenderedRule } from '../../../../data/rules.data'
import type { RuleDomain }   from '../../../../lib/shared/registries'

import SurfaceCardBase       from './SurfaceCardBase.vue'

const props = defineProps<{
  bodyHtml : string
  domain   : RuleDomain
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
    :domain="domain"
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

<style scoped>
.tab-index {
  display        : flex;
  flex-direction : column;
  gap            : 12px;
}

.tab-row {
  display        : flex;
  align-items    : center;
  gap            : 16px;
  border-bottom  : 1px solid color-mix(in srgb, var(--vp-c-text-3) 22%, transparent);
  padding-bottom : 6px;
}

.tab {
  position             : relative;
  padding              : 2px 0;
  font-family          : var(--vp-font-family-mono);
  font-size            : 0.72rem;
  letter-spacing       : 0.06em;
  text-decoration-line : none;
  color                : var(--vp-c-text-3);
  transition           : color 220ms cubic-bezier(0.4, 0, 0.2, 1);
}

.tab::after {
  content         : "";
  position        : absolute;
  left            : 0;
  right           : 0;
  bottom          : -7px;
  height          : 2px;
  background      : var(--domain-color);
  border-radius   : 2px;
  transform       : scaleX(0);
  transform-origin: center;
  transition      : transform 220ms cubic-bezier(0.4, 0, 0.2, 1);
}

.tab:hover,
.tab:focus-visible,
.tab.active {
  color : var(--domain-color);
}

.tab:hover::after,
.tab:focus-visible::after,
.tab.active::after {
  transform : scaleX(1);
}

.tab-label {
  min-height : 1.2em;
}

.tab-label-link {
  font-family          : var(--vp-font-family-mono);
  font-size            : 0.85rem;
  color                : var(--vp-c-text-1);
  text-decoration-line : none;
  transition           : color 220ms cubic-bezier(0.4, 0, 0.2, 1);
}

.tab-label-link:hover,
.tab-label-link:focus-visible {
  color : var(--domain-color);
}

.tab-swap-enter-active,
.tab-swap-leave-active {
  transition :
    opacity   180ms cubic-bezier(0.4, 0, 0.2, 1),
    transform 220ms cubic-bezier(0.4, 0, 0.2, 1);
}
.tab-swap-enter-from { opacity: 0; transform: translateY(2px); }
.tab-swap-leave-to   { opacity: 0; transform: translateY(-2px); }

@media (prefers-reduced-motion: reduce) {
  .tab, .tab::after { transition: none; }
}
</style>
