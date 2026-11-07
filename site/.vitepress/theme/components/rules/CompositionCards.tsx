import { useResizeObserver }                                    from '@vueuse/core'
import { mergeProps }                                           from '@zag-js/vue'
import { useData }                                              from 'vitepress'
import { computed, defineComponent, nextTick, ref, shallowRef } from 'vue'

import { Disclosure }           from '../../../kit'
import type { DisclosureScope } from '../../../kit'
import { FixtureNoChange }      from '../fixtures/FixtureNoChange'
import { FixturePairDoc }       from '../fixtures/FixturePairDoc'
import { FixtureToggle }        from '../fixtures/FixtureToggle'
import { RuleSegmentChip }      from './RuleSegmentChip'

import { useHashOpen }                            from '../../../lib/composables/use-hash-open'
import { COMPOSITION_RULE, fixtureEntry }         from '../../../lib/fixtures/entry'
import type { FixtureEntry }                      from '../../../lib/fixtures/entry'
import type { CompositionCase }                   from '../../../lib/rules/composition'
import { deckTravel }                             from '../../../lib/rules/deck-travel'
import { pageCompositionCases, type RuleSegment } from '../../../lib/rules/rule-view'
import { data as rules }                          from '../../../lib/rules/rules.data'
import type { FixtureTab }                        from '../../../lib/shared/fixture-tab'
import { InlineProse }                            from '../base/InlineProse'

interface CardRow extends FixtureEntry {
  case     : string
  segments : readonly RuleSegment[]
}

export const CompositionCards = defineComponent({
  name  : 'CompositionCards',
  props : {
    rule : String
  },
  setup(props) {
    const { frontmatter } = useData()

    function toCardRow(entry: CompositionCase): CardRow {
      return {
        ...fixtureEntry(frontmatter.value, COMPOSITION_RULE, entry.case),
        case     : entry.case,
        segments : entry.rules.map((slug, idx) => {
          const rule = rules.bySlug[slug] ?? null
          return { family: rule?.family ?? null, index: idx + 1, rule, slug }
        })
      }
    }

    // A rule page renders the subset the rule participates in, and every page
    // leaves out the cases it embeds inline. Each card leaves its folio empty so
    // the `prose-fixture-folio` counter numbers it in place, continuing the run the
    // fixtures above it started.
    const cards = computed<readonly CardRow[]>(() =>
      pageCompositionCases(frontmatter.value, props.rule).map(toCardRow))

    const activeCase = ref<string | null>(null)
    const activeTab  = ref<FixtureTab>('after')

    const list = shallowRef<HTMLElement | null>(null)
    const bars = shallowRef<HTMLElement[] | null>(null)

    function toggle(row: CardRow): void {
      void deckTravel(list.value!, async () => {
        activeCase.value = activeCase.value === row.case ? null : row.case
        await nextTick()
      })
    }

    // Holds each card's foot open for its deck, writing the height the deck takes
    // once its boxes spread and name their rules.
    useResizeObserver(bars, entries => {
      for (const { target } of entries) {
        target.closest<HTMLElement>('li.fixture-card')?.style.setProperty('--deck-bar-h', `${(target as HTMLElement).offsetHeight}px`)
      }
    })

    useHashOpen(fragment => {
      if (cards.value.some(row => row.case === fragment)) activeCase.value = fragment
    })

    return () => (
      <ol
        aria-label="Composition cards"
        class="composition-cards-list m-0 list-none p-0"
        ref={list}
      >
        {cards.value.map(row => (
          <Disclosure key={row.case} onUpdate:open={() => toggle(row)} open={activeCase.value === row.case}>
            {{
              default: (disclosure: DisclosureScope) => (
                <li
                  class="fixture-card group/fixture cursor-pointer"
                  data-edits={row.changesSource}
                  data-open={activeCase.value === row.case || undefined}
                  id={row.case}
                  onClick={() => toggle(row)}
                >
                  <div class="fixture-card-summary-row max-mobile:group-data-open/fixture:gap-y-0">
                    <span
                      aria-hidden="true"
                      class="composition-cards-slot col-1 row-1 mt-7 ml-8 h-20 self-start w-32"
                      style={{ '--deck-count': row.segments.length }}
                    />
                    <button
                      {...mergeProps(disclosure.trigger, { onClick: (event: MouseEvent) => event.stopPropagation() })}
                      class="fixture-card-summary col-2 row-1 max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-1 max-mobile:group-data-open/fixture:-col-end-1"
                      type="button"
                    >
                      <span aria-hidden="true" class="fixture-card-num" />
                      <span class="fixture-card-title text-balance in-code:normal-case" innerHTML={row.titleHtml} />
                    </button>
                    <div class="contents">
                      <ol
                        aria-label={`${row.segments.length} rules in pipeline order`}
                        class="composition-cards-ticks m-0 list-none p-0 z-2 flex"
                        style={{ '--deck-count': row.segments.length }}
                      >
                        {row.segments.map((seg, at) => (
                          <li
                            class="composition-cards-tick-item my-0 list-none ps-0 relative flex"
                            key={seg.slug}
                            onClick={event => event.stopPropagation()}
                            style={{ '--deck-at': at }}
                          >
                            <RuleSegmentChip segment={seg} withTooltip={false} />
                          </li>
                        ))}
                      </ol>
                      <div
                        class="col-3 row-1 inline-flex items-center max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-2 max-mobile:group-data-open/fixture:-col-end-1 max-mobile:group-data-open/fixture:min-h-0 max-mobile:group-data-open/fixture:justify-start max-mobile:group-data-open/fixture:justify-self-start max-mobile:group-data-open/fixture:p-0 max-mobile:group-data-open/fixture:pb-8"
                        onClick={event => event.stopPropagation()}
                        style={{ display: activeCase.value === row.case ? undefined : 'none' }}
                      >
                        {row.hasToggle
                          ? <FixtureToggle modelValue={activeTab.value} onUpdate:modelValue={next => { activeTab.value = next }} />
                          : <FixtureNoChange />}
                      </div>
                    </div>
                  </div>
                  <div
                    {...mergeProps(disclosure.content, { onClick: (event: MouseEvent) => event.stopPropagation() })}
                    class="fixture-card-body transition-grid-template-rows duration-grow ease-morph"
                    hidden={undefined}
                  >
                    <div class="fixture-card-body-inner">
                      <div class="fixture-card-body-content">
                        {row.descriptionNodes ? (
                          <>
                            <div class="fixture-card-desc flush-ends">
                              <InlineProse nodes={row.descriptionNodes} />
                            </div>
                            <div aria-hidden="true" class="fixture-card-rule" />
                          </>
                        ) : null}
                        {activeCase.value === row.case ? (
                          <FixturePairDoc
                            activeTab={activeTab.value}
                            configHtml={row.configHtml}
                            inputHtml={row.inputHtml}
                            outputHtml={row.outputHtml}
                          />
                        ) : null}
                        <ol
                          aria-hidden="true"
                          class="composition-cards-bar invisible mt-12 mb-0 flex list-none flex-wrap gap-6 p-0"
                          inert
                          ref={bars}
                          ref_for
                        >
                          {row.segments.map(seg => (
                            <li class="my-0 flex min-w-0 flex-none list-none ps-0" key={seg.slug}>
                              <RuleSegmentChip segment={seg} withTooltip={false} />
                            </li>
                          ))}
                        </ol>
                      </div>
                    </div>
                  </div>
                </li>
              )
            }}
          </Disclosure>
        ))}
      </ol>
    )
  }
})
