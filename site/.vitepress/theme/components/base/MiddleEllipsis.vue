<script setup lang="ts">
import { useResizeObserver }                  from '@vueuse/core'
import { onMounted, ref, useTemplateRef, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    tail ?: number
    text  : string
  }>(),
  { tail : 3 }
)

const elRef   = useTemplateRef<HTMLSpanElement>('el')
const display = ref(props.text)

function fit() {
  const el = elRef.value
  if (!el) return
  el.textContent = props.text
  if (el.scrollWidth <= el.clientWidth + 1) { display.value = props.text; return }
  if (props.text.length <= props.tail + 1)  { display.value = props.text; return }

  let lo   = 0
  let hi   = props.text.length - props.tail - 1
  let best = -1
  while (lo <= hi) {
    const m        = Math.floor((lo + hi) / 2)
    el.textContent = `${props.text.slice(0, m)}…${props.text.slice(-props.tail)}`
    if (el.scrollWidth <= el.clientWidth + 1) { best = m; lo = m + 1 }
    else                                       { hi = m - 1 }
  }
  display.value = best < 1
    ? `…${props.text.slice(-props.tail)}`
    : `${props.text.slice(0, best)}…${props.text.slice(-props.tail)}`
}

onMounted(async () => {
  if (document.fonts?.ready) {
    await document.fonts.ready
  }
  fit()
})

useResizeObserver(elRef, fit)
watch(() => props.text, fit)
</script>

<template><span ref="el" class="middle-ellipsis">{{ display }}</span></template>
