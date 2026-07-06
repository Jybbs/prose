<script setup lang="ts">
import { ref, useTemplateRef, watch } from 'vue'

import { useSettledMeasure } from '../../../lib/composables/use-settled-measure'
import { middleEllipsis }    from '../../../lib/shared/middle-ellipsis'

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
  const fits = (candidate: string): boolean => {
    el.textContent = candidate
    return el.scrollWidth <= el.clientWidth + 1
  }
  display.value = middleEllipsis(fits, props.tail, props.text)
  // The probes above mutated the span directly, so restore the chosen text
  // even when the ref value is unchanged and Vue skips the patch.
  el.textContent = display.value
}

useSettledMeasure(elRef, fit)
watch(() => props.text, fit)
</script>

<template><span ref="el" class="middle-ellipsis">{{ display }}</span></template>
