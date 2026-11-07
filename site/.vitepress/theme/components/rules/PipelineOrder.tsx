import { onKeyStroke }                           from '@vueuse/core'
import { computed, defineComponent, shallowRef } from 'vue'

import { orderRelation }    from '../../../lib/rules/order-line'
import { passCensus }       from '../../../lib/rules/pipeline-census'
import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { data as rules }    from '../../../lib/rules/rules.data'
import { familyEmblem }     from '../../../lib/shared/emblems'
import { formatFolio }      from '../../../lib/shared/numerals'
import { FAMILY_META }      from '../../../lib/shared/registries'

import { MiddleEllipsis, NibBadge } from '../../../kit'

export const PipelineOrder = defineComponent({
  name: 'PipelineOrder',
  setup() {
    const families = pipeline.rules.map(rule => rule.family)

    // Holds the pass a reader points at or focuses, which keeps every pass up to
    // it as at rest and grays every later one until the Escape key clears it.
    const here   = shallowRef<number | null>(null)
    const census = computed(() => passCensus(families, here.value))

    // Names how each rule stands to the pointed one, which it runs after, which
    // run after it, or the pointed rule itself.
    const order = computed(() => {
      if (here.value === null) return []
      const chosen = pipeline.rules[here.value]
      return pipeline.rules.map(rule => orderRelation(chosen, rule))
    })

    const placed = (at: number) => ({ 'is-here': at === here.value, 'is-later': here.value !== null && at > here.value })

    function focus(event: FocusEvent, at: number): void {
      if ((event.target as Element).matches(':focus-visible')) here.value = at
    }

    onKeyStroke('Escape', () => { here.value = null })

    return () => (
      <section
        aria-label="Pipeline order"
        class="pipeline-order my-32 panel bg-bg-soft px-32 py-22"
      >
        <header class="mb-22 flex flex-wrap items-center justify-between gap-12">
          <div aria-label="Passes per family" class="flex flex-wrap gap-4" role="group">
            {census.value.map(entry => (
              <span
                aria-label={`${FAMILY_META[entry.family].label}, ${entry.count} rules`}
                class={['pipeline-order-census-chip nib-ground inline-flex items-center gap-5 border-1 border-divider px-6 py-2 font-mono text-label leading-1-2 font-400 uppercase tabular-nums tracking-normal', { 'is-later': entry.count === 0 }]}
                data-family={entry.family}
                key={entry.family}
                role="img"
              >
                <NibBadge emblem={familyEmblem(entry.family)} size={13} />
                <span aria-hidden="true">{entry.count}</span>
              </span>
            ))}
          </div>
          <span class="pipeline-order-edition ml-auto kicker in-code:tracking-normal in-code:normal-case">
            {here.value === null
              ? (
                <>
                  {pipeline.rules.length}
                  {' '}
                  rules
                </>
              )
              : (
              <>
                pass
                {' '}
                {formatFolio(here.value + 1)}
                {' '}
                of
                {' '}
                {pipeline.rules.length}
              </>
            )}
            {' '}
            ·
            {' '}
            <code>crate/src/rules/registry.rs</code>
          </span>
          <ol
            aria-hidden="true"
            class="pipeline-order-spine m-0 flex h-10 shrink-0 grow basis-full list-none items-end gap-2 p-0"
          >
            {pipeline.rules.map((rule, at) => (
              <li
                class={['m-0 flex h-full flex-1 items-end', placed(at)]}
                key={rule.slug}
                onPointerenter={() => { here.value = at }}
              />
            ))}
          </ol>
        </header>
        <ol class="m-0 columns-2 gap-x-40 p-0 column-rule-divider max-medium:columns-1">
          {pipeline.rules.map((rule, at) => (
            <li
              class={['pipeline-order-entry mt-0 mb-2 list-none break-inside-avoid', placed(at)]}
              data-family={rule.family}
              key={rule.slug}
            >
              <a
                class="pipeline-order-link flex items-center gap-12 rounded-6 px-8 py-9 text-text-1 no-underline transition-colors hover:text-brand-2"
                data-rule={rule.documented ? rule.slug : undefined}
                href={rule.documented ? rules.bySlug[rule.slug].href : undefined}
                onFocusin={event => focus(event, at)}
                onPointerenter={() => { here.value = at }}
                title={rule.title}
              >
                <span aria-hidden="true" class="pipeline-order-gutter w-10 shrink-0 text-center font-mono text-sm" data-order={order.value[at]}>
                  {order.value[at] === 'earlier' ? '↑' : order.value[at] === 'later' ? '↓' : null}
                </span>
                <span class="folio">№ {formatFolio(rule.position)}</span>
                <MiddleEllipsis
                  class="pipeline-order-name min-w-0 font-display text-0-92rem font-500 italic"
                  tail={2}
                  text={rule.slug}
                />
                <span aria-hidden="true" class="leader-fill" />
                {rule.family
                  ? <NibBadge emblem={familyEmblem(rule.family)} size={15.2} />
                  : <span aria-hidden="true" class="pipeline-order-glyph shrink-0 text-md">·</span>}
              </a>
            </li>
          ))}
        </ol>
      </section>
    )
  }
})
