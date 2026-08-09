<script setup lang="ts">
import { useAriaHidden }     from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { formatFolio }       from '../../../../lib/shared/numerals'

const props = defineProps<{
  active   : boolean
  distance : number
  index    : number
  rule     : RenderedRule
}>()

const emit = defineEmits<{ select: [] }>()

const ariaHidden = useAriaHidden()
</script>

<template>
  <a
    class="surface-pip"
    :class="{ active }"
    :style="{ '--d': distance }"
    :href="rule.href"
    :aria-label="rule.slug"
    :tabindex="ariaHidden ? -1 : undefined"
    @focus="emit('select')"
    @mouseenter="emit('select')"
  ><span class="surface-pip-mark" aria-hidden="true" /><span class="surface-pip-folio">{{ formatFolio(index + 1) }}</span></a>
</template>
