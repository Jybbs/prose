<script setup lang="ts">
import { computed, ref, watchEffect } from 'vue'
import { inBrowser } from 'vitepress'
import { autoUpdate, flip, offset, shift, useFloating } from '@floating-ui/vue'

import { data as glossary } from '../../data/glossary.data'

const props = defineProps<{ term: string }>()

const entry = computed(() => glossary[props.term])
if (!entry.value) {
  throw new Error(
    `Glossary term "${props.term}" not found. ` +
    `Available terms: ${Object.keys(glossary).sort().join(', ')}`
  )
}

const anchor  = ref<HTMLElement | null>(null)
const floater = ref<HTMLElement | null>(null)
const isOpen  = ref(false)
let   timer   : number | null = null

const { floatingStyles } = useFloating(anchor, floater, {
  placement           : 'top',
  middleware          : [offset(10), flip(), shift({ padding: 8 })],
  whileElementsMounted: autoUpdate
})

function open() {
  if (timer !== null) {
    clearTimeout(timer)
    timer = null
  }
  isOpen.value = true
}

function scheduleClose() {
  if (timer !== null) clearTimeout(timer)
  timer = window.setTimeout(() => {
    isOpen.value = false
    timer        = null
  }, 140)
}

function toggle() {
  if (isOpen.value) scheduleClose()
  else              open()
}

watchEffect(onCleanup => {
  if (!inBrowser || !isOpen.value) return
  const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') isOpen.value = false }
  document.addEventListener('keydown', handler)
  onCleanup(() => document.removeEventListener('keydown', handler))
})
</script>

<template>
  <span
    ref="anchor"
    class="glossary-anchor"
    @mouseenter="open"
    @mouseleave="scheduleClose"
    @focusin="open"
    @focusout="scheduleClose"
    @click="toggle"
    tabindex="0"
  >
    <slot>{{ term }}</slot>
  </span>
  <Teleport to="body">
    <span
      v-if="isOpen"
      ref="floater"
      :style="floatingStyles"
      class="glossary-tooltip"
      role="tooltip"
      @mouseenter="open"
      @mouseleave="scheduleClose"
    >
      <span class="glossary-tooltip-title">{{ term }}</span>
      <span class="glossary-tooltip-divider" aria-hidden="true" />
      <span class="glossary-tooltip-body" v-html="entry?.definitionHtml" />
      <a
        v-if="entry?.href"
        :href="entry.href"
        class="glossary-tooltip-link"
        @click="isOpen = false"
      >Read more →</a>
    </span>
  </Teleport>
</template>
