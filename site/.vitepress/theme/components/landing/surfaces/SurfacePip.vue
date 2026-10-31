<script setup lang="ts">
import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { formatFolio }       from '../../../../lib/shared/numerals'

const props = defineProps<{
  active   : boolean
  distance : number
  index    : number
  rule     : RenderedRule
}>()

const emit = defineEmits<{ select: [] }>()

const tabindex = useHiddenTabindex()
</script>

<template>
  <a
    class="surface-pip group/pip relative grid place-items-center focus-visible:rounded-pill focus-visible:outline-offset-1"
    :class="{ active }"
    :style="{ '--d': distance }"
    :href="rule.href"
    :aria-label="rule.slug"
    :tabindex="tabindex"
    @focus="emit('select')"
    @mouseenter="emit('select')"
  ><span
    class="surface-pip-mark relative z-1 rounded-pill"
    :class="active ? 'bg-family-22-bg-soft' : 'bg-family/45 group-hover/pip:bg-family-22-bg-soft'"
    aria-hidden="true"
  /><span
    class="surface-pip-folio absolute inset-0 z-2 grid shrink-0 place-items-center pt-1 font-mono text-meta leading-1 tracking-folio uppercase tabular-nums"
    :class="active
      ? 'translate-y-0 text-family opacity-100'
      : 'translate-y-3 text-text-3 opacity-0 group-hover/pip:translate-y-0 group-hover/pip:text-family group-hover/pip:opacity-100'"
  >{{ formatFolio(index + 1) }}</span></a>
</template>
