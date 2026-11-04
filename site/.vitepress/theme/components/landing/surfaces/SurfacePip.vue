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
    :data-active="active || undefined"
    :style="{ '--d': distance }"
    :href="rule.href"
    :aria-label="rule.slug"
    :tabindex="tabindex"
    @focus="emit('select')"
    @mouseenter="emit('select')"
  ><span
    class="surface-pip-mark relative z-1 rounded-pill bg-family/45 group-hover/pip:bg-family-22-bg-soft group-data-active/pip:bg-family-22-bg-soft motion-reduce:delay-0"
    aria-hidden="true"
  /><span
    class="surface-pip-folio absolute inset-0 z-2 grid shrink-0 translate-y-3 place-items-center pt-1 font-mono text-meta leading-none tracking-folio text-text-3 uppercase tabular-nums opacity-0 group-hover/pip:translate-y-0 group-hover/pip:text-family group-hover/pip:opacity-100 group-data-active/pip:translate-y-0 group-data-active/pip:text-family group-data-active/pip:opacity-100 motion-reduce:delay-0"
  >{{ formatFolio(index + 1) }}</span></a>
</template>
