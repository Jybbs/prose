import { defineComponent, Fragment } from 'vue'

import { useTabSelect }  from '../../../lib/composables/use-tab-select'
import { data as codes } from '../../../lib/exit-codes/exit-codes.data'
import { InlineProse }   from '../base/InlineProse'

export const ExitCodeMatrix = defineComponent({
  name: 'ExitCodeMatrix',
  setup() {
    const { active: selectedRow, selected } = useTabSelect(codes, c => c.code)

    // Opens each outcome's band on the first code that reports it.
    const heads = (at: number) => at === 0 || codes[at - 1]!.outcome !== codes[at]!.outcome

    return () => (
      <div class="my-1-5rem grid grid-cols-280-1fr items-center gap-24 max-tablet:grid-cols-1fr">
        <nav
          aria-label="Exit codes"
          class="flex flex-col gap-4 py-4 pr-18 pl-4 not-max-tablet:border-r-1 not-max-tablet:border-r-divider max-tablet:border-b-1 max-tablet:border-r-0 max-tablet:border-b-divider max-tablet:pe-0 max-tablet:pb-16"
        >
          {codes.map((row, at) => (
            <Fragment key={row.code}>
              {heads(at) && (
                <header
                  class="exit-codes-band-head flex items-center gap-9 px-12 pt-8"
                  data-outcome={row.outcome}
                >
                  <span class="band-name">{row.band}</span>
                  <span aria-hidden="true" class="h-2 grow bg-outcome-mark" />
                </header>
              )}
              <button
                aria-current={row.code === selected.value || undefined}
                class="group exit-code-index-row grid grid-cols-24-auto-1fr items-baseline gap-14 border-none p-12 text-left text-text-2 hover:text-text-1 focus-visible:focus-ring focus-visible:-outline-offset-1 aria-current:text-text-1"
                data-outcome={row.outcome}
                onFocus={() => { selected.value = row.code }}
                onMouseenter={() => { selected.value = row.code }}
                type="button"
              >
                <span class="text-right font-mono text-1rem transition-colors group-hover:text-outcome-ink group-aria-current:text-outcome-ink motion-reduce:transition-none lining-nums tabular-nums">{row.code}</span>
                <span class="font-display text-md italic">{row.label}</span>
                <span aria-hidden="true" class="mx-4 leader" />
              </button>
            </Fragment>
          ))}
        </nav>
        <article
          aria-live="polite"
          class="ps-18 max-tablet:ps-16"
          data-outcome={selectedRow.value.outcome}
        >
          <header class="mb-18 flex items-center gap-14">
            <span
              aria-hidden="true"
              class="exit-code-entry-numeral font-mono text-2-6rem leading-none font-500 text-outcome-ink lining-nums tabular-nums"
            >{selectedRow.value.code}</span>
            <span class="exit-code-entry-label font-display text-1-5rem leading-none font-500 text-text-1">{selectedRow.value.label}</span>
          </header>
          <p class="exit-code-entry-summary max-w-60ch font-base text-md text-text-1 in-code:whitespace-nowrap">{selectedRow.value.summary}</p>
          <dl class="exit-code-entry-facts m-0 grid max-w-60ch grid-cols-max-1fr border-t-1 border-t-divider in-code:whitespace-nowrap">
            {selectedRow.value.facts.map(fact => (
              <Fragment key={fact.label}>
                <dt class="m-0 border-b-1 border-b-divider py-8 pr-16 kicker leading-24 font-400 text-text-2">{fact.label}</dt>
                <dd class="m-0 border-b-1 border-b-divider py-8 font-base text-0-84rem leading-1-55 text-text-1">
                  <InlineProse nodes={fact.nodes} />
                </dd>
              </Fragment>
            ))}
          </dl>
        </article>
      </div>
    )
  }
})
