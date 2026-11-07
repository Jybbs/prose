import { defineComponent } from 'vue'

import { InlineProse } from '../base/InlineProse'
import { FamilyChip }  from '../../../kit'

import { useGlossaryFolio } from '../../../lib/composables/use-glossary-folio'
import { BODY_LINK_CLASS }  from '../../../lib/shared/constants'
import { formatFolio }      from '../../../lib/shared/numerals'

const STEP_CLASS       = 'inline-flex size-24 cursor-pointer items-center justify-center rounded-sm border-1 border-divider bg-bg p-0 font-mono text-1rem leading-none font-400 text-text-3 transition-colors hover:text-text-1 hover:border-text-3'
const STEP_GLYPH_CLASS = 'block -translate-y-0.5 leading-none'

export const GlossaryFolioPane = defineComponent({
  name: 'GlossaryFolioPane',
  setup() {
    const { active, activeIndex, filtered, step } = useGlossaryFolio()

    return () => (
      <div data-family={active.value?.primaryFamily}>
        {active.value && (
          <article class="relative mt-24 min-w-0 panel bg-bg pt-16 pr-28 pb-18 pl-32">
            <header class="-mt-16 -mr-28 mb-18 -ml-32 flex min-h-36 items-center justify-between gap-12 rounded-t-base-inner border-b-1 border-b-divider bg-bg-soft p-6">
              <p class="glossary-folio-folio m-0 kicker flex flex-wrap items-center gap-x-10 gap-y-6 pl-6">
                {active.value.families.map(family => (
                  <FamilyChip
                    class="border-0 bg-transparent pl-2"
                    family={family}
                    key={family}
                    linked={false}
                  />
                ))}
                <span
                  aria-hidden="true"
                  class="inline-flex h-24 items-center justify-center leading-none text-divider"
                >·</span>
                <span class="inline-flex h-24 items-center justify-center leading-none">{activeIndex.value < 0 ? '–' : formatFolio(activeIndex.value + 1)} / {formatFolio(filtered.value.length)}</span>
              </p>
              <div class="inline-flex h-24 shrink-0 items-center gap-6">
                <button
                  aria-label="Previous entry"
                  class={STEP_CLASS}
                  onClick={() => step(-1)}
                  type="button"
                >
                  <span aria-hidden="true" class={STEP_GLYPH_CLASS}>‹</span>
                </button>
                <button
                  aria-label="Next entry"
                  class={STEP_CLASS}
                  onClick={() => step(1)}
                  type="button"
                >
                  <span aria-hidden="true" class={STEP_GLYPH_CLASS}>›</span>
                </button>
              </div>
            </header>
            <h2 class="glossary-folio-headword relative mt-0 mb-14 border-t-0 pt-0 font-display text-1-7rem leading-1-15 font-500 text-text-1 *:relative *:inline-block">
              {active.value.href
                ? <a class={BODY_LINK_CLASS} href={active.value.href}>{active.value.slug}</a>
                : <span class="relative inline-block after:absolute after:inset-x-0 after:-bottom-2 after:h-1 after:translate-y-4 after:bg-text-3 after:opacity-0 after:transition-translate-and-opacity hover:after:translate-y-0 hover:after:opacity-100">{active.value.slug}</span>}
            </h2>
            {active.value.aliases.length > 0 && (
              <p class="mt-0 flex flex-wrap items-center gap-x-8 gap-y-6 border-b-1 border-b-divider pb-18">
                <span class="mr-8 font-mono text-label tracking-label uppercase text-text-2">also</span>
                {active.value.aliases.map(alias => (
                  <code
                    class="inline-flex items-center rounded-sm border-none bg-bg-alt px-10 py-4 align-baseline text-meta leading-1-3 tracking-meta text-text-2"
                    key={alias}
                  >{alias}</code>
                ))}
              </p>
            )}
            <div class="text-md leading-1-55 text-text-1">
              <InlineProse nodes={active.value.definitionNodes} />
            </div>
          </article>
        )}
      </div>
    )
  }
})
