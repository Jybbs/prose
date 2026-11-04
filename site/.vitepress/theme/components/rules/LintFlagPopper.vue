<script setup lang="ts">
import { PopperWrapper }                    from 'floating-vue'
import { computed, ref, shallowRef, watch } from 'vue'

import RuleCard from './RuleCard.vue'

import { usePopperAnchor }                  from '../../../lib/composables/use-popper-anchor'
import { lintShorthand, type Shorthand }    from '../../../lib/fixtures/lint-shorthand'
import { data as rules, type RenderedRule } from '../../../lib/rules/rules.data'
import { highlight }                        from '../../../lib/shared/highlight'
import { inlineCode }                       from '../../../lib/shared/inline-code'
import { latestRun }                        from '../../../lib/shared/latest-run'

interface ActiveFinding {
  message   : string
  rule      : RenderedRule
  shorthand : Shorthand | null
}

interface BlockPanes {
  after  : string
  before : string
}

const BLOCK_SIDES: readonly { label: string, side: keyof BlockPanes }[] = [
  { label: 'Before', side: 'before' },
  { label: 'After',  side: 'after'  }
]

const { aim, key, reference, target } = usePopperAnchor()

const active      = ref<ActiveFinding | null>(null)
const block       = shallowRef<BlockPanes | null>(null)
const messageHtml = computed(() => inlineCode(active.value?.message ?? ''))

const run = latestRun()

// The `.lint-flag` anchors sit inside `v-html` static HTML, so event
// delegation finds the hovered flag and the popper takes it as a dynamic
// reference node rather than wrapping each anchor in a component. Floating
// resolves that reference once per mount, so `usePopperAnchor` keys the popper
// to a fresh instance whenever the flag changes, pinning it to the hovered span
// rather than the panel it caches on first show.
function show(event: Event): void {
  const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
  const slug = flag?.dataset.rule ?? ''
  const rule = rules.bySlug[slug]
  if (!flag || !rule) return
  // Crossing between a flag's own token spans re-fires the delegated hover,
  // so a repeat on the flag already shown returns before the render and the
  // highlight.
  if (active.value && flag === target.value) return
  const message    = flag.dataset.message ?? ''
  aim(flag)
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
    :key="key"
    theme="rule-card"
    placement="bottom-start"
    popper-class="lint-popover"
    auto-boundary-max-size
    :auto-hide="false"
    :distance="6"
    :handle-resize="false"
    :overflow-padding="16"
    :popper-triggers="[]"
    :reference-node="reference"
    :shown="active !== null"
    :triggers="[]"
  >
    <template #popper>
      <RuleCard v-if="active" :rule="active.rule" :clickable="false">
        <template #header>
          <span v-if="active.shorthand?.kind === 'replace'" class="flex flex-wrap items-center gap-0-4rem font-mono text-xs">
            <span class="lint-chip-struck rounded-sm bg-text-1/8 px-0-45rem py-0-1rem text-text-3 line-through">{{ active.shorthand.before }}</span>
            <span class="text-text-3" aria-hidden="true">→</span>
            <span class="lint-chip-suggest rounded-sm px-0-45rem py-0-1rem bg-text-1/12 text-text-1">{{ active.shorthand.after }}</span>
          </span>
          <span v-else-if="active.shorthand?.kind === 'remove'" class="flex flex-wrap items-center gap-0-4rem font-mono text-xs">
            <span class="lint-chip-struck rounded-sm bg-text-1/8 px-0-45rem py-0-1rem text-text-3 line-through">{{ active.shorthand.text }}</span>
          </span>
          <span v-else-if="active.shorthand?.kind === 'insert'" class="flex flex-wrap items-center gap-0-4rem font-mono text-xs">
            <span class="lint-chip-insert rounded-sm bg-text-1/8 px-0-45rem py-0-1rem text-text-2">{{ active.shorthand.anchor }}<ins class="lint-inserted no-underline bg-text-1/18 text-text-1">{{ active.shorthand.inserted }}</ins></span>
          </span>
          <div v-else-if="active.shorthand?.kind === 'block' && block" class="lint-block flex min-h-0 flex-col gap-6">
            <div
              v-for="pane in BLOCK_SIDES"
              :key="pane.side"
              class="lint-block-side flex min-h-0 flex-col overflow-hidden rounded-sm border-1 bg-code-block border-divider"
              :data-side="pane.side"
            >
              <span class="lint-block-label border-b-1 border-b-divider/40 px-12 py-3 font-mono text-label tracking-label uppercase bg-code-block text-text-2">{{ pane.label }}</span>
              <div class="lint-block-code code-text min-h-0 overflow-auto px-12 py-8" v-html="block[pane.side]" />
            </div>
          </div>
          <span v-else class="lint-message text-sm" v-html="messageHtml" />
        </template>
      </RuleCard>
    </template>
  </PopperWrapper>
</template>
