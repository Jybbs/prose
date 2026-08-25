<script setup lang="ts">
import { useEventListener }            from '@vueuse/core'
import { options, PopperWrapper }      from 'floating-vue'
import { onBeforeUnmount, shallowRef } from 'vue'

import RuleCard from './RuleCard.vue'

import { usePopperAnchor }                  from '../../../lib/composables/use-popper-anchor'
import { data as rules, type RenderedRule } from '../../../lib/rules/rules.data'

interface PopperTheme {
  delay : {
    hide : number
    show : number
  }
}

const themes = (options as { themes: Record<string, PopperTheme> }).themes
const delay  = themes['rule-card'].delay

const { aim, key, reference, target } = usePopperAnchor()

const rule = shallowRef<RenderedRule | null>(null)

let timer: ReturnType<typeof setTimeout> | undefined

// One delegated popper serves every `a.rule-link[data-rule]` in the page,
// taking the hovered anchor as its reference node.
function linkFrom(event: Event): HTMLElement | null {
  const node = event.target
  return node instanceof Element ? node.closest<HTMLElement>('a.rule-link[data-rule]') : null
}

function show(event: Event): void {
  const link = linkFrom(event)
  if (!link) return
  const found = rules.bySlug[link.dataset.rule ?? '']
  if (!found) return
  clearTimeout(timer)
  if (link === target.value && rule.value) return
  timer = setTimeout(() => {
    aim(link)
    rule.value = found
  }, delay.show)
}

function hide(event: Event): void {
  if (!linkFrom(event)) return
  clearTimeout(timer)
  timer = setTimeout(() => { rule.value = null }, delay.hide)
}

useEventListener('focusin',   show)
useEventListener('focusout',  hide)
useEventListener('mouseover', show)
useEventListener('mouseout',  hide)

onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <PopperWrapper
    :key="key"
    theme="rule-card"
    :auto-hide="false"
    :no-auto-focus="true"
    :popper-class="`rule-card-popper fam-${rule?.family ?? ''}`"
    :popper-triggers="[]"
    :reference-node="reference"
    :shown="rule !== null"
    :triggers="[]"
  >
    <template #popper>
      <RuleCard v-if="rule" :rule="rule" :clickable="false" />
    </template>
  </PopperWrapper>
</template>
