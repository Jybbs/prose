import { computed, defineComponent, ref, shallowRef, watch } from 'vue'

import { Popup, ScrollRegion } from '../../../kit'
import { RuleCard }            from './RuleCard'

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

const CHIP = 'rounded-sm bg-text-1/8 px-0-45rem py-0-1rem text-text-2'

const SHORTHAND = 'flex flex-wrap items-center gap-0-4rem font-mono text-xs'

export interface LintFlagPopupHandle {
  hide   : () => void
  toggle : (event: Event) => boolean
}

export const LintFlagPopup = defineComponent({
  name: 'LintFlagPopup',
  setup(_props, { expose }) {
    const active      = ref<ActiveFinding | null>(null)
    const block       = shallowRef<BlockPanes | null>(null)
    const messageHtml = computed(() => inlineCode(active.value?.message ?? ''))
    const target      = shallowRef<HTMLElement | null>(null)

    const run = latestRun()

    // The `.lint-flag` anchors sit inside HTML set through `innerHTML`, so event
    // delegation finds the pressed flag and the popup takes it as its anchor
    // rather than wrapping each anchor in a component. Reports whether the event
    // landed on a flag, which closes the card that flag has open.
    function toggle(event: Event): boolean {
      const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
      const slug = flag?.dataset.rule ?? ''
      const rule = rules.bySlug[slug]
      if (!flag || !rule) return false
      event.preventDefault()
      if (active.value && flag === target.value) {
        hide()
        return true
      }
      const message = flag.dataset.message ?? ''
      target.value  = flag
      active.value  = {
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
      return true
    }

    function hide(): void {
      active.value = null
    }

    // A replacement too tall for a chip renders through the same client
    // highlighter the sandbox paints with, so the suggested Python reads as
    // Python. A newer flag supersedes the render in flight, and the message
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

    expose({ hide, toggle })

    return () => (
      <Popup
        anchor={target.value}
        class="lint-popover"
        gutter={6}
        label={active.value?.rule.name ?? ''}
        onDismiss={hide}
        open={active.value !== null}
        overflowPadding={16}
        theme="rule-card"
      >
        {{ default: () => {
          const finding = active.value
          const panes   = block.value
          return finding && (
            <RuleCard clickable={false} rule={finding.rule}>
              {{
                header: () => (
                  finding.shorthand?.kind === 'replace'
                    ? (
                      <span class={SHORTHAND}>
                        <span class={`lint-chip-struck ${CHIP} line-through`}>{finding.shorthand.before}</span>
                        <span aria-hidden="true" class="text-text-3">→</span>
                        <span class="lint-chip-suggest rounded-sm px-0-45rem py-0-1rem bg-text-1/12 text-text-1">{finding.shorthand.after}</span>
                      </span>
                    )
                    : finding.shorthand?.kind === 'remove'
                    ? (
                      <span class={SHORTHAND}>
                        <span class={`lint-chip-struck ${CHIP} line-through`}>{finding.shorthand.text}</span>
                      </span>
                    )
                    : finding.shorthand?.kind === 'insert'
                    ? (
                      <span class={SHORTHAND}>
                        <span class={`lint-chip-insert ${CHIP}`}>
                          {finding.shorthand.anchor}
                          <ins class="lint-inserted no-underline bg-text-1/18 text-text-1">{finding.shorthand.inserted}</ins>
                        </span>
                      </span>
                    )
                    : finding.shorthand?.kind === 'block' && panes
                    ? (
                      <div class="lint-block flex min-h-0 flex-col gap-6">
                        {BLOCK_SIDES.map(pane => (
                          <div
                            class="lint-block-side flex min-h-0 flex-col overflow-hidden rounded-sm border-1 bg-code-block border-divider"
                            data-side={pane.side}
                            key={pane.side}
                          >
                            <span class="lint-block-label border-b-1 border-b-divider/40 px-12 py-3 font-mono text-label tracking-label uppercase bg-code-block text-text-2">{pane.label}</span>
                            <ScrollRegion
                              class="lint-block-code code-text min-h-0 overflow-auto px-12 py-8"
                              html={panes[pane.side]}
                            />
                          </div>
                        ))}
                      </div>
                    )
                    : <span class="text-sm in-code:text-0-86em in-code:text-code-ink" innerHTML={messageHtml.value} />
                )
              }}
            </RuleCard>
          )
        } }}
      </Popup>
    )
  }
})
