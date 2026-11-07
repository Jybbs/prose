import { defineComponent, nextTick, ref } from 'vue'

import { Disclosure, NibBadge, PermalinkAnchor } from '../../../kit'
import type { DisclosureScope }                  from '../../../kit'
import { InlineRuleLink }                        from '../rules/InlineRuleLink'

import { useHashOpen }         from '../../../lib/composables/use-hash-open'
import { data as facetGroups } from '../../../lib/reference/facets.data'
import { familyEmblem }        from '../../../lib/shared/emblems'
import { counted }             from '../../../lib/shared/numerals'
import { InlineProse }         from '../base/InlineProse'

export const PerRuleFacets = defineComponent({
  name: 'PerRuleFacets',
  setup() {
    const open       = ref<Record<string, boolean>>({})
    const toggle     = (family: string): void => { open.value[family] = !open.value[family] }
    const facetCount = (family: (typeof facetGroups)[number]): string =>
      counted(family.rules.reduce((sum, group) => sum + group.facets.length, 0), 'facet')

    // The family each anchor sits under, so a hash naming a rule or one of its
    // facets expands the section that holds it.
    const familyOf = new Map(facetGroups.flatMap(family =>
      family.rules.flatMap(group =>
        [group.anchor, ...group.facets.map(facet => facet.anchor)]
          .map(anchor => [anchor, family.family] as const))))

    // The browser reaches a collapsed target while it is still hidden and scrolls
    // nowhere, so the section expands first and the scroll follows the paint.
    useHashOpen(fragment => {
      const family = familyOf.get(fragment)
      if (family === undefined) return
      open.value[family] = true
      void nextTick(() => document.querySelector(`[id="${fragment}"]`)?.scrollIntoView())
    })

    return () => (
      <div class="my-1-5rem flex flex-col gap-6">
        {facetGroups.map(family => (
          <section
            class="per-rule-facets-section relative"
            data-family={family.family}
            key={family.family}
          >
            <Disclosure onUpdate:open={() => toggle(family.family)} open={open.value[family.family] ?? false}>
              {{
                default: (disclosure: DisclosureScope) => (
                  <>
                    <button
                      {...disclosure.trigger}
                      class="per-rule-facets-head group flex w-full items-center gap-9 border-0 border-l-2 py-7 pl-14 text-left border-l-transparent focus-visible:focus-ring focus-visible:outline-offset-2"
                      type="button"
                    >
                      <span
                        aria-hidden="true"
                        class="size-0 flex-none border-t-4 border-b-4 border-l-5 border-t-transparent border-b-transparent group-aria-expanded:rotate-90 per-rule-facets-chevron border-l-text-2 group-hover:border-l-text-1 group-focus-visible:border-l-text-1"
                      />
                      <span class="per-rule-facets-label kicker text-text-2 font-600">
                        {family.family !== 'generic' ? <NibBadge emblem={familyEmblem(family.family)} size={13.6} /> : null}
                        {' '}
                        {family.label}
                        {' '}
                        <span class="per-rule-facets-count before:mx-0-5em before:content-middot text-text-2 font-400">{facetCount(family)}</span>
                      </span>
                    </button>
                    <div {...disclosure.content} class="pt-10 pb-2">
                      {family.rules.map(group => (
                        <div class="border-l-2 pl-14 not-first:mt-18 border-l-transparent" key={group.rule}>
                          <p class="per-rule-facets-rule-head mt-0 mb-10 -ml-14 flex" id={group.anchor}>
                            {family.family !== 'generic'
                              ? <InlineRuleLink slug={group.rule} />
                              : <span class="per-rule-facets-scope inline-block rounded-r-sm px-10 py-3 font-mono text-0-85em bg-chip-ground text-text-1">{group.rule}</span>}
                            <PermalinkAnchor anchor={group.anchor} label={group.rule} />
                          </p>
                          <dl class="flex flex-col">
                            {group.facets.map(facet => (
                              <div
                                class="per-rule-facets-entry py-10 not-first:border-t-1 not-first:border-t-divider first:pt-0 last:pb-0"
                                key={facet.key}
                              >
                                <dt
                                  class="per-rule-facets-term flex flex-wrap items-center gap-x-0-75rem gap-y-0-4rem"
                                  id={facet.anchor}
                                >
                                  <span class="per-rule-facets-key font-mono text-md font-600 text-text-1">{facet.key}</span>
                                  <PermalinkAnchor anchor={facet.anchor} label={facet.key} />
                                  <span class="ml-auto inline-flex items-center gap-0-4rem">
                                    <span class="per-rule-facets-type rounded-sm bg-bg-soft px-0-5em py-2 font-mono text-xs text-text-2">{facet.type}</span>
                                    <span class="inline-flex items-baseline gap-0-4em rounded-sm px-0-5em py-2 text-xs bg-chip-ground">
                                      <span class="font-mono text-kicker tracking-label uppercase text-text-2">default</span>
                                      <span class="per-rule-facets-default-value font-mono text-text-1">{facet.default}</span>
                                    </span>
                                  </span>
                                </dt>
                                <dd class="per-rule-facets-meaning mt-6 max-w-62ch pl-0-9rem text-sm leading-1-6 text-text-2 max-mobile:pl-0">
                                  <InlineProse nodes={facet.meaningNodes} />
                                </dd>
                              </div>
                            ))}
                          </dl>
                        </div>
                      ))}
                    </div>
                  </>
                )
              }}
            </Disclosure>
          </section>
        ))}
      </div>
    )
  }
})
