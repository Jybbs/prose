import { useToggle }                      from '@vueuse/core'
import { mergeProps }                     from '@zag-js/vue'
import { computed, defineComponent, ref } from 'vue'

import { Disclosure, NibBadge } from '../../../kit'
import type { DisclosureScope } from '../../../kit'
import { FixtureConfig }        from './FixtureConfig'
import { FixtureNoChange }      from './FixtureNoChange'
import { FixturePairDoc }       from './FixturePairDoc'
import { FixtureToggle }        from './FixtureToggle'

import { useFixtureEntry } from '../../../lib/composables/use-fixture-entry'
import { useHashOpen }     from '../../../lib/composables/use-hash-open'
import { data as rules }   from '../../../lib/rules/rules.data'
import { familyEmblem }    from '../../../lib/shared/emblems'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'
import { ruleSlug }        from '../../../lib/shared/rule-slug'
import { InlineProse }     from '../base/InlineProse'

export const FixtureCard = defineComponent({
  name  : 'FixtureCard',
  props : {
    case : { required: true, type: String },
    rule : { required: true, type: String }
  },
  setup(props) {
    const entry     = useFixtureEntry(props)
    const id        = computed(() => `fixture-${props.rule}-${props.case}`)
    const activeTab = ref<FixtureTab>('after')

    const family = computed(() => rules.bySlug[ruleSlug(props.rule)]?.family ?? null)

    const [isOpen, toggle] = useToggle(false)

    useHashOpen(fragment => { if (fragment === id.value) isOpen.value = true })

    return () => (
      <Disclosure onUpdate:open={next => { isOpen.value = next }} open={isOpen.value}>
        {{
          default: (disclosure: DisclosureScope) => (
            <section
              class="fixture-card group/fixture"
              data-edits={entry.value.changesSource}
              data-family={family.value}
              data-lint={entry.value.hasFindings}
              data-open={isOpen.value || undefined}
              id={id.value}
            >
              <div
                class="fixture-card-summary-row max-mobile:group-data-open/fixture:gap-y-0"
                onClick={() => toggle()}
              >
                <span
                  aria-hidden="true"
                  class="mt-7 mr-12 ml-8 flex size-20 items-center justify-center self-start max-mobile:mr-0"
                >
                  {family.value ? (
                    <NibBadge
                      class="block text-family-nib-card group-data-[edits=false]/fixture:not-group-data-[lint=true]/fixture:text-text-3-card-31"
                      emblem={familyEmblem(family.value)}
                      size={14}
                    />
                  ) : null}
                </span>
                <button
                  {...mergeProps(disclosure.trigger, { onClick: (event: MouseEvent) => event.stopPropagation() })}
                  class="fixture-card-summary max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-1 max-mobile:group-data-open/fixture:-col-end-1"
                  type="button"
                >
                  <span aria-hidden="true" class="fixture-card-num" />
                  <span class="fixture-card-title in-code:normal-case" innerHTML={entry.value.titleHtml} />
                </button>
                <div
                  class="hidden items-center group-data-open/fixture:inline-flex group-data-[edits=false]/fixture:not-group-data-[lint=true]/fixture:inline-flex max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-2 max-mobile:group-data-open/fixture:-col-end-1 max-mobile:group-data-open/fixture:min-h-0 max-mobile:group-data-open/fixture:justify-start max-mobile:group-data-open/fixture:justify-self-start max-mobile:group-data-open/fixture:p-0 max-mobile:group-data-open/fixture:pb-8"
                  onClick={event => event.stopPropagation()}
                >
                  {entry.value.hasToggle
                    ? <FixtureToggle modelValue={activeTab.value} onUpdate:modelValue={next => { activeTab.value = next }} />
                    : <FixtureNoChange />}
                </div>
              </div>
              <div
                {...disclosure.content}
                class="fixture-card-body transition-grid-template-rows"
                hidden={undefined}
              >
                <div class="fixture-card-body-inner">
                  <div class="fixture-card-body-content">
                    {entry.value.descriptionNodes ? (
                      <>
                        <div class="fixture-card-desc flush-ends">
                          <InlineProse nodes={entry.value.descriptionNodes} />
                        </div>
                        <div aria-hidden="true" class="fixture-card-rule" />
                      </>
                    ) : null}
                    {entry.value.configHtml ? <FixtureConfig html={entry.value.configHtml} /> : null}
                    {isOpen.value ? (
                      <FixturePairDoc
                        activeTab={activeTab.value}
                        inputHtml={entry.value.inputHtml}
                        outputHtml={entry.value.outputHtml}
                        sandboxSeed={entry.value.sandboxSeed}
                      />
                    ) : null}
                  </div>
                </div>
              </div>
            </section>
          )
        }}
      </Disclosure>
    )
  }
})
