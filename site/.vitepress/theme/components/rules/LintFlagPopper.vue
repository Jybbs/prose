<script setup lang="ts">
import { PopperWrapper }  from 'floating-vue'
import { computed, ref, shallowRef } from 'vue'

import RuleCard from './RuleCard.vue'

import { data as rules }     from '../../../lib/rules/rules.data'
import type { RenderedRule } from '../../../lib/rules/rules.data'
import { lintShorthand }     from '../../../lib/fixtures/lint-shorthand'
import type { Shorthand }    from '../../../lib/fixtures/lint-shorthand'
import { inlineCode }        from '../../../lib/shared/inline-code'

interface ActiveFinding {
  message   : string
  rule      : RenderedRule
  shorthand : Shorthand | null
}

const active      = ref<ActiveFinding | null>(null)
const activeFlag  = shallowRef<HTMLElement | null>(null)
const anchor      = ref(0)
const messageHtml = computed(() => inlineCode(active.value?.message ?? ''))

// The `.lint-flag` anchors sit inside `v-html` static HTML, so event
// delegation finds the hovered flag and the popper takes it as a dynamic
// reference node rather than wrapping each anchor in a component. Floating
// resolves that reference once per mount, so `anchor` keys the popper to a
// fresh instance whenever the flag changes, pinning it to the hovered span
// rather than the panel it caches on first show.
function show(event: Event): void {
  const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
  const rule = flag?.dataset.rule ? rules.bySlug[flag.dataset.rule] : undefined
  if (!flag || !rule) return
  const message    = flag.dataset.message ?? ''
  if (flag !== activeFlag.value) anchor.value += 1
  activeFlag.value = flag
  active.value = {
    message,
    rule,
    shorthand : lintShorthand({
      before    : flag.dataset.before,
      flagged   : flag.textContent ?? '',
      message,
      rule      : flag.dataset.rule ?? '',
      suggested : flag.dataset.suggested
    })
  }
}

function hide(): void {
  active.value = null
}

defineExpose({ hide, show })
</script>

<template>
  <PopperWrapper
    :key="anchor"
    theme="rule-card"
    placement="bottom-start"
    popper-class="lint-popover fam-lint"
    :auto-hide="false"
    :distance="6"
    :handle-resize="false"
    :popper-triggers="[]"
    :reference-node="() => activeFlag!"
    :shown="active !== null"
    :triggers="[]"
  >
    <template #popper>
      <RuleCard v-if="active" :rule="active.rule" :clickable="false">
        <template #header>
          <span v-if="active.shorthand?.kind === 'replace'" class="lint-shorthand">
            <span class="lint-chip lint-chip-struck">{{ active.shorthand.before }}</span>
            <span class="lint-into" aria-hidden="true">→</span>
            <span class="lint-chip lint-chip-suggest">{{ active.shorthand.after }}</span>
          </span>
          <span v-else-if="active.shorthand?.kind === 'remove'" class="lint-shorthand">
            <span class="lint-chip lint-chip-struck">{{ active.shorthand.text }}</span>
          </span>
          <span v-else class="lint-message" v-html="messageHtml" />
        </template>
      </RuleCard>
    </template>
  </PopperWrapper>
</template>
