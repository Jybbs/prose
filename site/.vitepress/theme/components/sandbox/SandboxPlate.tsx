import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import { RuleCard } from '../rules/RuleCard'

import type { Facet, RuleControl } from '../../../lib/composables/use-chip-panel'
import type { ProseSandbox }       from '../../../lib/composables/use-prose-sandbox'
import type { RenderedRule }       from '../../../lib/rules/rules.data'

export const SandboxPlate = defineComponent({
  name  : 'SandboxPlate',
  props : {
    card    : { required: true, type: [Object, null] as PropType<RenderedRule | null> },
    facets  : { required: true, type: Array as PropType<readonly Facet[]> },
    rule    : { required: true, type: Object as PropType<RuleControl> },
    sandbox : { required: true, type: Object as PropType<ProseSandbox> }
  },
  emits : { close: () => true },
  setup(props, { emit }) {
    function boolValue(facet: Facet): boolean {
      return props.sandbox.facetValue(props.rule.slug, facet) === true
    }

    function numberValue(facet: Facet): number {
      return props.sandbox.facetValue(props.rule.slug, facet) as number
    }

    function textValue(facet: Facet): string {
      const value = props.sandbox.facetValue(props.rule.slug, facet)
      return Array.isArray(value) ? value.join(', ') : String(value)
    }

    function writeText(facet: Facet, raw: string): void {
      const value = facet.kind === 'stringList'
        ? raw.split(',').map(part => part.trim()).filter(Boolean)
        : raw
      props.sandbox.setFacet(props.rule.slug, facet, value)
    }

    return () => (
      <section
        aria-label={`Settings for ${props.rule.slug}`}
        class="sandbox-plate w-full min-w-0 rounded-sm border-1 border-text-1-40-divider bg-text-1-4-panel-wash-card"
        data-family={props.rule.family || null}
      >
        <div class="relative">
          {props.card
            ? <RuleCard clickable={false} rule={props.card} />
            : (
            <div class="px-16 py-12">
              <span class="font-mono text-kicker-sm tracking-kicker text-text-1 uppercase">{props.rule.slug}</span>
            </div>
          )}
          <button
            aria-label="Close"
            class="sandbox-plate-close absolute top-1/2 right-16 grid size-26 animate-plate-settle place-items-center rounded-sm border-1 border-divider bg-bg text-text-3 transition-colors hover:border-text-1-55-divider hover:text-text-1 focus-visible:outline-offset-1"
            onClick={() => emit('close')}
            type="button"
          >
            <svg aria-hidden="true" class="glyph" fill="none" viewBox="0 0 24 24">
              <path d="M6 6l12 12" />
              <path d="M18 6L6 18" />
            </svg>
          </button>
        </div>
        <div
          aria-hidden="true"
          class="relative mx-16 h-1 origin-left scale-x-0 animate-plate-draw bg-divider after:absolute after:-top-1 after:left-0 after:h-2 after:w-30 after:bg-text-1 motion-reduce:scale-x-100"
        />
        <div class="grid animate-plate-settle grid-cols-max-1fr-max gap-x-1-1rem px-16 pb-8 max-medium:grid-cols-1fr-max">
          {props.facets.map(facet => (
            <div
              class="col-span-full grid min-h-2-1rem grid-cols-subgrid items-center not-first:border-t-1 not-first:border-t-divider max-medium:gap-y-0-1rem max-medium:py-0-4rem"
              key={facet.key}
            >
              <span class="font-mono text-xxs font-500 tracking-0-02em text-text-1 uppercase max-medium:col-start-1 max-medium:row-start-1">{facet.label}</span>
              <span
                class="sandbox-plate-hint py-0-25rem text-xs leading-1-4 text-text-3 italic max-medium:col-start-1 max-medium:row-start-2 max-medium:py-0"
                innerHTML={facet.hintHtml}
              />
              {facet.kind === 'bool'
                ? (
                  <button
                    aria-checked={boolValue(facet)}
                    aria-label={facet.label}
                    class="group/switch relative h-22 w-40 justify-self-end rounded-pill border-none bg-text-3/40 transition-colors focus-visible:outline-offset-1 aria-checked:bg-text-1 max-medium:col-2 max-medium:row-span-2 max-medium:row-start-1"
                    onClick={() => props.sandbox.setFacet(props.rule.slug, facet, !boolValue(facet))}
                    role="switch"
                    type="button"
                  >
                    <span class="absolute top-3 left-3 size-16 rounded-half bg-bg transition-transform group-aria-checked/switch:translate-x-18" />
                  </button>
                )
                : facet.kind === 'int'
                ? (
                  <input
                    aria-label={facet.label}
                    class="panel-number h-22 w-40 justify-self-end rounded-sm border-1 border-text-1-55-divider bg-bg font-mono text-xs leading-20 text-text-1 tabular-nums focus-visible:outline-offset-1 max-medium:col-2 max-medium:row-span-2 max-medium:row-start-1"
                    onInput={event => props.sandbox.setFacet(props.rule.slug, facet, Number((event.target as HTMLInputElement).value))}
                    type="number"
                    value={numberValue(facet)}
                  />
                )
                : (
                <input
                  aria-label={facet.label}
                  class="h-22 w-9rem justify-self-end rounded-sm border-1 border-text-1-55-divider bg-bg px-0-4rem font-mono text-xs text-text-1 focus-visible:outline-offset-1 max-medium:col-2 max-medium:row-span-2 max-medium:row-start-1"
                  onChange={event => writeText(facet, (event.target as HTMLInputElement).value)}
                  type="text"
                  value={textValue(facet)}
                />
              )}
            </div>
          ))}
        </div>
      </section>
    )
  }
})
