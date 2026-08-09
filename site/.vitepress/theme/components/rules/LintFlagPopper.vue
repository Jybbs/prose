<script setup lang="ts">
import { PopperWrapper }                    from 'floating-vue'
import { computed, ref, shallowRef, watch } from 'vue'

import RuleCard from './RuleCard.vue'

import { lintShorthand, type Shorthand }     from '../../../lib/fixtures/lint-shorthand'
import { data as rules, type RenderedRule }  from '../../../lib/rules/rules.data'
import { highlight }                         from '../../../lib/shared/highlight'
import { inlineCode }                        from '../../../lib/shared/inline-code'
import { latestRun }                         from '../../../lib/shared/latest-run'

interface ActiveFinding {
  message   : string
  rule      : RenderedRule
  shorthand : Shorthand | null
}

interface BlockPanes {
  after  : string
  before : string
}

const active      = ref<ActiveFinding | null>(null)
const activeFlag  = shallowRef<HTMLElement | null>(null)
const anchor      = ref(0)
const block       = shallowRef<BlockPanes | null>(null)
const messageHtml = computed(() => inlineCode(active.value?.message ?? ''))

const run = latestRun()

// The `.lint-flag` anchors sit inside `v-html` static HTML, so event
// delegation finds the hovered flag and the popper takes it as a dynamic
// reference node rather than wrapping each anchor in a component. Floating
// resolves that reference once per mount, so `anchor` keys the popper to a
// fresh instance whenever the flag changes, pinning it to the hovered span
// rather than the panel it caches on first show.
function show(event: Event): void {
  const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
  const slug = flag?.dataset.rule ?? ''
  const rule = rules.bySlug[slug]
  if (!flag || !rule) return
  // Crossing between a flag's own token spans re-fires the delegated hover,
  // so a repeat on the flag already shown returns before the render and the
  // highlight.
  if (active.value && flag === activeFlag.value) return
  const message    = flag.dataset.message ?? ''
  if (flag !== activeFlag.value) anchor.value += 1
  activeFlag.value = flag
  active.value = {
    message,
    rule,
    shorthand : lintShorthand({
      flagged   : flag.textContent ?? '',
      message,
      replaced  : flag.dataset.replaced,
      rule      : slug,
      suggested : flag.dataset.suggested
    })
  }
}

function hide(): void {
  active.value = null
}

// A replacement too tall for a chip renders through the same client
// highlighter the sandbox paints with, so the suggested Python reads as
// Python. A newer hover supersedes the render in flight, and the message
// holds the header until the panes land.
watch(active, async current => {
  const shorthand = current?.shorthand
  if (shorthand?.kind !== 'block') {
    run.cancel()
    block.value = null
    return
  }
  const superseded      = run.begin()
  const [after, before] = await Promise.all([
    highlight(shorthand.after,  'python'),
    highlight(shorthand.before, 'python')
  ])
  if (superseded()) return
  block.value = { after, before }
})

defineExpose({ hide, show })
</script>

<template>
  <PopperWrapper
    :key="anchor"
    theme="rule-card"
    placement="bottom-start"
    popper-class="lint-popover fam-lint"
    auto-boundary-max-size
    :auto-hide="false"
    :distance="6"
    :handle-resize="false"
    :overflow-padding="16"
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
          <span v-else-if="active.shorthand?.kind === 'insert'" class="lint-shorthand">
            <span class="lint-chip lint-chip-insert">{{ active.shorthand.anchor }}<ins class="lint-inserted">{{ active.shorthand.inserted }}</ins></span>
          </span>
          <div v-else-if="active.shorthand?.kind === 'block' && block" class="lint-block">
            <div class="lint-block-side" data-side="before">
              <span class="lint-block-label">Before</span>
              <div class="code-panel-code lint-block-code" v-html="block.before" />
            </div>
            <div class="lint-block-side" data-side="after">
              <span class="lint-block-label">After</span>
              <div class="code-panel-code lint-block-code" v-html="block.after" />
            </div>
          </div>
          <span v-else class="lint-message" v-html="messageHtml" />
        </template>
      </RuleCard>
    </template>
  </PopperWrapper>
</template>
