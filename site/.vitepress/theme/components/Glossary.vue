<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { autoUpdate, flip, offset, shift, useFloating } from '@floating-ui/vue'

import { glossary } from '../../lib/glossary'

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
const timer   = ref<number | null>(null)

const { floatingStyles } = useFloating(anchor, floater, {
  placement           : 'top',
  middleware          : [offset(10), flip(), shift({ padding: 8 })],
  whileElementsMounted: autoUpdate
})

function open() {
  if (timer.value !== null) {
    clearTimeout(timer.value)
    timer.value = null
  }
  isOpen.value = true
}

function scheduleClose() {
  if (timer.value !== null) clearTimeout(timer.value)
  timer.value = window.setTimeout(() => {
    isOpen.value = false
    timer.value  = null
  }, 140)
}

function toggle() {
  if (isOpen.value) scheduleClose()
  else              open()
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    isOpen.value = false
  }
}

watch(isOpen, value => {
  if (typeof document === 'undefined') return
  if (value) document.addEventListener('keydown', onKeydown)
  else       document.removeEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  if (typeof document !== 'undefined') document.removeEventListener('keydown', onKeydown)
  if (timer.value !== null)            clearTimeout(timer.value)
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
      <span class="glossary-tooltip-body" v-html="entry?.definition" />
      <a
        v-if="entry?.href"
        :href="entry.href"
        class="glossary-tooltip-link"
        @click="isOpen = false"
      >Read more →</a>
    </span>
  </Teleport>
</template>
