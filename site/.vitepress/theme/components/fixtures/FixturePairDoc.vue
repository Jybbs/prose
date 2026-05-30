<script setup lang="ts">
import { computed, ref } from 'vue'

import RuleCard from '../rules/RuleCard.vue'

import { data as rules }                          from '../../../data/rules.data'
import type { RenderedRule }                      from '../../../data/rules.data'
import { lintShorthand, LOOSE_CONSTANT_HOMES }    from '../../../lib/fixtures/lint-shorthand'
import type { Shorthand }                         from '../../../lib/fixtures/lint-shorthand'
import type { FixtureTab }                        from '../../../lib/shared/fixture-tab'

defineProps<{
  activeTab  : FixtureTab
  inputHtml  : string
  outputHtml : string
}>()

interface ActiveFinding {
  left      : number
  message   : string
  rule      : RenderedRule
  shorthand : Shorthand | null
  top       : number
}

const homes       = LOOSE_CONSTANT_HOMES
const active      = ref<ActiveFinding | null>(null)
const messageHtml = computed(() =>
  (active.value?.message ?? '').replace(/`([^`]+)`/g, '<code>$1</code>')
)

function show(event: Event): void {
  const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
  const rule = flag?.dataset.rule ? rules.bySlug[flag.dataset.rule] : undefined
  if (!flag || !rule) return
  const rect    = flag.getBoundingClientRect()
  const message = flag.dataset.message ?? ''
  active.value = {
    left      : rect.left,
    message,
    rule,
    shorthand : lintShorthand({
      before    : flag.dataset.before,
      flagged   : flag.textContent ?? '',
      message,
      rule      : flag.dataset.rule ?? '',
      suggested : flag.dataset.suggested
    }),
    top       : rect.bottom + 6
  }
}

function hide(): void {
  active.value = null
}
</script>

<template>
  <div class="fixture-pair fixture-pair-doc">
    <div
      class="fixture-pair-panel"
      @mouseover="show"
      @mouseout="hide"
      @focusin="show"
      @focusout="hide"
      v-html="activeTab === 'before' ? inputHtml : outputHtml"
    />
    <Teleport to="body">
      <div
        v-if="active"
        class="lint-popover v-popper--theme-rule-card fam-lint"
        :style="{ left: `${active.left}px`, top: `${active.top}px` }"
      >
        <RuleCard :rule="active.rule" :clickable="false">
          <template #header>
            <span v-if="active.shorthand?.kind === 'replace'" class="lint-shorthand">
              <span class="lint-chip lint-chip-struck">{{ active.shorthand.before }}</span>
              <span class="lint-into" aria-hidden="true">→</span>
              <span class="lint-chip lint-chip-suggest">{{ active.shorthand.after }}</span>
            </span>
            <span v-else-if="active.shorthand?.kind === 'relocate'" class="lint-shorthand">
              <span class="lint-chip lint-chip-gray">{{ active.shorthand.name }}</span>
              <span class="lint-relocate" aria-hidden="true">⤴</span>
              <span class="lint-slot">
                <span v-for="home in homes" :key="home.parent" class="lint-slot-chip"><span :class="{ 'lint-slot-keyword': home.keyword }">{{ home.parent }}</span><span class="lint-slot-bracket" aria-hidden="true">⟨</span><span class="lint-slot-leaf">{{ home.leaf }}</span><span class="lint-slot-bracket" aria-hidden="true">⟩</span></span>
              </span>
            </span>
            <span v-else-if="active.shorthand?.kind === 'remove'" class="lint-shorthand">
              <span class="lint-chip lint-chip-struck">{{ active.shorthand.text }}</span>
            </span>
            <span v-else class="lint-message" v-html="messageHtml" />
          </template>
        </RuleCard>
      </div>
    </Teleport>
  </div>
</template>
