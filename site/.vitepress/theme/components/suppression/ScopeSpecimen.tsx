import { defineComponent } from 'vue'

import { ScrollRegion } from '../../../kit'

import { lookup }                                from '../../../lib/shared/lookup'
import { data as directives }                    from '../../../lib/suppression/directives.data'
import { directiveHref, SCOPE_META, scopeBands } from '../../../lib/suppression/scopes'
import type { ScopeKey }                         from '../../../lib/suppression/scopes'

interface SpecimenLine {
  bracket : 'close' | 'mid' | 'open' | 'solo'
  scope   : ScopeKey
  text    : string
}

const LEGEND_IDS = [
  'prose-off', 'fmt-off', 'fmt-skip', 'prose-skip-rules', 'prose-ignore-rules', 'prose-keep'
]

export const ScopeSpecimen = defineComponent({
  name: 'ScopeSpecimen',
  setup() {
    const lines: SpecimenLine[] = [
      { bracket : 'open',  scope : 'file',      text : '# prose: off' },
      { bracket : 'mid',   scope : 'file',      text : '' },
      { bracket : 'mid',   scope : 'file',      text : 'from collections import OrderedDict' },
      { bracket : 'mid',   scope : 'file',      text : '' },
      { bracket : 'mid',   scope : 'file',      text : 'def render(rows):' },
      { bracket : 'open',  scope : 'block',     text : '    # fmt: off' },
      { bracket : 'mid',   scope : 'block',     text : '    grid = [[1, 0, 0, 0],' },
      { bracket : 'mid',   scope : 'block',     text : '            [0, 1, 0, 0],' },
      { bracket : 'mid',   scope : 'block',     text : '            [0, 0, 1, 0],' },
      { bracket : 'mid',   scope : 'block',     text : '            [0, 0, 0, 1]]' },
      { bracket : 'close', scope : 'block',     text : '    # fmt: on' },
      { bracket : 'mid',   scope : 'file',      text : '' },
      { bracket : 'open',  scope : 'line',      text : '    out = build(' },
      { bracket : 'mid',   scope : 'line',      text : '        rows,' },
      { bracket : 'close', scope : 'line',      text : '    )  # fmt: skip' },
      { bracket : 'solo',  scope : 'line',      text : '    log(out)  # prose: ignore[<rule>]' },
      { bracket : 'mid',   scope : 'file',      text : '' },
      { bracket : 'open',  scope : 'construct', text : '    STAGES = {  # prose: keep' },
      { bracket : 'mid',   scope : 'construct', text : '        "fetch"    : fetch,' },
      { bracket : 'mid',   scope : 'construct', text : '        "parse"    : parse,' },
      { bracket : 'mid',   scope : 'construct', text : '        "validate" : validate,' },
      { bracket : 'mid',   scope : 'construct', text : '        "render"   : render,' },
      { bracket : 'close', scope : 'construct', text : '    }' },
      { bracket : 'close', scope : 'file',      text : '' }
    ]

    const byId = Object.fromEntries(directives.map(directive => [directive.id, directive]))

    const entries = LEGEND_IDS.map(id => {
      const directive = lookup(byId, id, 'Directive')
      return {
        form  : directive.pairRole === 'opens' && directive.pairId !== undefined
          ? `${directive.form} … ${lookup(byId, directive.pairId, 'Directive').form}`
          : directive.form,
        href  : directiveHref(directive.scope),
        id    : id,
        scope : directive.scope
      }
    })

    const legend = scopeBands(entries)

    return () => (
      <div class="my-1-5rem grid grid-cols-1fr-180-220 gap-24 panel bg-panel-wash px-24 py-22 max-tablet:grid-cols-1fr">
        <ScrollRegion as="pre" class="overflow-x-auto rounded-sm border-1 border-divider bg-bg px-8 py-14 text-0-8rem leading-1-55">
          <code class="block">
            {lines.map((line, idx) => (
              <span
                class="scope-specimen-line group grid grid-cols-28-14-1fr items-center gap-0"
                data-bracket={line.bracket}
                data-scope={line.scope}
                key={idx}
              >
                <span aria-hidden="true" class="pr-8 text-right text-xxs text-text-3 select-none">{String(idx + 1).padStart(2, ' ')}</span>
                <span
                  aria-hidden="true"
                  class="relative h-full w-14 before:absolute before:inset-y-0 before:left-3 before:w-3 before:bg-scope-70-divider group-data-[bracket=solo]:before:inset-y-3/10 group-data-[bracket=close]:after:absolute group-data-[bracket=close]:after:bottom-0 group-data-[bracket=close]:after:left-1 group-data-[bracket=close]:after:h-3 group-data-[bracket=close]:after:w-8 group-data-[bracket=close]:after:rounded-1 group-data-[bracket=close]:after:bg-scope-70-divider group-data-[bracket=open]:after:absolute group-data-[bracket=open]:after:top-0 group-data-[bracket=open]:after:left-1 group-data-[bracket=open]:after:h-3 group-data-[bracket=open]:after:w-8 group-data-[bracket=open]:after:rounded-1 group-data-[bracket=open]:after:bg-scope-70-divider group-data-[bracket=solo]:after:absolute group-data-[bracket=solo]:after:top-1/2 group-data-[bracket=solo]:after:left-0.5 group-data-[bracket=solo]:after:size-8 group-data-[bracket=solo]:after:-translate-y-1/2 group-data-[bracket=solo]:after:rounded-half group-data-[bracket=solo]:after:bg-scope-80-bg"
                />
                <span class="text-text-1 group-data-[bracket=open]:text-scope-ink group-data-[bracket=close]:text-scope-ink group-data-[bracket=solo]:text-scope-ink">{line.text || ' '}</span>
              </span>
            ))}
          </code>
        </ScrollRegion>
        <ul class="m-0 flex list-none flex-col gap-12 self-start pl-0">
          {legend.map(band => (
            <li
              class="scope-specimen-legend-row flex flex-col gap-4 rounded-sm border-1 border-l-3 border-scope-25-divider border-l-scope-70-divider bg-scope-6-bg px-10 pt-6 pb-8"
              data-scope={band.scope}
              key={band.scope}
            >
              <span aria-hidden="true" class="flex items-center gap-8">
                <span class="pip size-18 bg-scope-85-bg text-0-74rem">{SCOPE_META[band.scope].pip}</span>
                <span class="font-mono text-meta tracking-kicker text-text-2 uppercase">{SCOPE_META[band.scope].label}</span>
              </span>
              <ul class="m-0 flex list-none flex-wrap gap-6 pl-0">
                {band.items.map(d => (
                  <li class="scope-specimen-legend-directive m-0" key={d.id}>
                    <a class="body-link" href={d.href}>
                      <code>{d.form}</code>
                    </a>
                  </li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      </div>
    )
  }
})
