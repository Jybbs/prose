<script setup lang="ts">
import { options, PopperWrapper }                      from 'floating-vue'
import { onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'

import RuleCard from './RuleCard.vue'

import { data as rules, type RenderedRule } from '../../../lib/rules/rules.data'

interface PopperTheme {
  delay : {
    hide : number
    show : number
  }
}

const themes = (options as { themes: Record<string, PopperTheme> }).themes
const delay  = themes['rule-card'].delay

const activeLink = shallowRef<HTMLElement | null>(null)
const anchor     = ref(0)
const rule       = shallowRef<RenderedRule | null>(null)

let timer: ReturnType<typeof setTimeout> | undefined

// One delegated popper serves every `a.rule-link[data-rule]` in the page,
// taking the hovered anchor as its reference node. `anchor` keys it to a fresh
// instance whenever that reference changes.
function linkFrom(event: Event): HTMLElement | null {
  const target = event.target
  return target instanceof Element ? target.closest<HTMLElement>('a.rule-link[data-rule]') : null
}

function show(event: Event): void {
  const link = linkFrom(event)
  if (!link) return
  const found = rules.bySlug[link.dataset.rule ?? '']
  if (!found) return
  clearTimeout(timer)
  if (link === activeLink.value && rule.value) return
  timer = setTimeout(() => {
    if (link !== activeLink.value) {
      anchor.value    += 1
      activeLink.value = link
    }
    rule.value = found
  }, delay.show)
}

function hide(event: Event): void {
  if (!linkFrom(event)) return
  clearTimeout(timer)
  timer = setTimeout(() => { rule.value = null }, delay.hide)
}

onMounted(() => {
  document.addEventListener('focusin',   show)
  document.addEventListener('focusout',  hide)
  document.addEventListener('mouseover', show)
  document.addEventListener('mouseout',  hide)
})

onBeforeUnmount(() => {
  clearTimeout(timer)
  document.removeEventListener('focusin',   show)
  document.removeEventListener('focusout',  hide)
  document.removeEventListener('mouseover', show)
  document.removeEventListener('mouseout',  hide)
})
</script>

<template>
  <PopperWrapper
    :key="anchor"
    theme="rule-card"
    :auto-hide="false"
    :no-auto-focus="true"
    :popper-class="`rule-card-popper fam-${rule?.family ?? ''}`"
    :popper-triggers="[]"
    :reference-node="() => activeLink!"
    :shown="rule !== null"
    :triggers="[]"
  >
    <template #popper>
      <RuleCard v-if="rule" :rule="rule" :clickable="false" />
    </template>
  </PopperWrapper>
</template>
