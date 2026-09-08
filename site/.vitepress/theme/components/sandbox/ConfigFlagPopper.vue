<script setup lang="ts">
import { PopperWrapper } from 'floating-vue'
import { computed, ref } from 'vue'

import { usePopperAnchor }         from '../../../lib/composables/use-popper-anchor'
import { noticeKey, rankedFacets } from '../../../lib/sandbox/config-decorations'
import type { RuleControl }        from '../../../lib/sandbox/config-schema.data'

const props = defineProps<{ rules: readonly RuleControl[] }>()
const emit  = defineEmits<{ apply: [path: string, replacement: string] }>()

const GRACE_MS = 120

const { aim, key, reference, target } = usePopperAnchor()

const path = ref('')

let closing: number | null = null

// A path that names a rule offers that rule's facets, closest spelling first.
// A root key names no rule, so it offers none.
const facets = computed(() => rankedFacets(path.value, props.rules))

// The path reads as a trail from the prose table down to the segment that
// failed, so the hierarchy shows what the key was written under.
const trail = computed(() => path.value.split('.').slice(0, -1))
const wrote = computed(() => path.value.split('.').at(-1) ?? '')

// The anchors sit inside `v-html` static HTML, so event delegation finds the
// hovered key and the popper takes it as a dynamic reference node. A repeat on
// the flag already shown returns before the read, since crossing between a
// flag's own token spans re-fires the delegated hover.
function show(event: Event): void {
  const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
  if (!flag) return
  hold()
  if (path.value && flag === target.value) return
  const named = noticeKey(flag.dataset.message ?? '')
  if (named === null) return
  aim(flag)
  path.value = named
}

// The pointer crosses dead space on its way from the key to a replacement
// button, so a leave schedules the close and entering the card cancels it.
function hide(): void {
  closing ??= window.setTimeout(() => { path.value = ''; closing = null }, GRACE_MS)
}

function hold(): void {
  if (closing === null) return
  window.clearTimeout(closing)
  closing = null
}

function close(): void {
  hold()
  path.value = ''
}

function apply(replacement: string): void {
  emit('apply', path.value, replacement)
  close()
}

defineExpose({ hide, show })
</script>

<template>
  <PopperWrapper
    :key="key"
    theme="glossary"
    placement="bottom-start"
    popper-class="config-notice-popover"
    auto-boundary-max-size
    :auto-hide="false"
    :distance="8"
    :handle-resize="false"
    :overflow-padding="16"
    :popper-triggers="['hover']"
    :reference-node="reference"
    :shown="path !== ''"
    :triggers="[]"
    @apply-hide="hide"
  >
    <template #popper>
      <div class="config-notice-card" @mouseenter="hold" @mouseleave="close">
        <p class="config-notice-trail">
          <span class="config-notice-root">[tool.prose]</span>
          <template v-for="segment in trail" :key="segment">
            <span class="config-notice-arrow" aria-hidden="true">›</span>
            <span>{{ segment }}</span>
          </template>
          <template v-if="!facets.length">
            <span class="config-notice-arrow" aria-hidden="true">›</span>
            <span class="config-notice-struck">{{ wrote }}</span>
          </template>
        </p>
        <div v-if="facets.length" class="config-notice-fixes">
          <button
            v-for="facet in facets"
            :key="facet.key"
            type="button"
            class="config-notice-fix"
            @click="apply(facet.key)"
          >
            <span class="config-notice-mark" aria-hidden="true">✓</span>
            <span>{{ facet.key }}</span>
            <span class="config-notice-default">{{ facet.kind }} · {{ facet.default }}</span>
          </button>
          <p class="config-notice-absent">
            <span class="config-notice-mark" aria-hidden="true">✕</span>
            <span class="config-notice-struck">{{ wrote }}</span>
          </p>
        </div>
      </div>
    </template>
  </PopperWrapper>
</template>
