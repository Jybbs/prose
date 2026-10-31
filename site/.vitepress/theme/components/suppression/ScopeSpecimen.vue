<script setup lang="ts">
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
</script>

<template>
  <div class="my-1-5rem grid grid-cols-1fr-180-220 gap-24 panel bg-panel-wash px-24 py-22 tablet:grid-cols-1fr">
    <pre class="overflow-x-auto rounded-sm border-1 border-divider bg-bg px-8 py-14 text-0-8rem leading-1-55"><code class="block"><span
      v-for="(line, idx) in lines"
      :key="idx"
      class="scope-specimen-line group grid grid-cols-28-14-1fr items-center gap-0"
      :data-scope="line.scope"
      :data-bracket="line.bracket"
    ><span class="pr-8 text-right text-xxs text-text-3 select-none" aria-hidden="true">{{ String(idx + 1).padStart(2, ' ') }}</span><span class="scope-specimen-bracket relative h-full w-14 before:absolute before:inset-y-0 before:left-3 before:w-3 before:bg-scope-70-divider group-bracket-solo:before:inset-y-3/10 group-bracket-close:after:bottom-0 group-bracket-end:after:absolute group-bracket-end:after:left-1 group-bracket-end:after:h-3 group-bracket-end:after:w-8 group-bracket-end:after:rounded-1 group-bracket-end:after:bg-scope-70-divider group-bracket-open:after:top-0" aria-hidden="true"></span><span class="text-text-1 group-bracket-end:text-scope-75-text-1 group-bracket-solo:text-scope-75-text-1">{{ line.text || ' ' }}</span></span></code></pre>

    <ul class="scope-specimen-legend">
      <li
        v-for="band in legend"
        :key="band.scope"
        class="scope-specimen-legend-row flex flex-col gap-4 rounded-sm border-1 border-l-3 border-scope-25-divider border-l-scope-70-divider bg-scope-6-bg px-10 pt-6 pb-8"
        :data-scope="band.scope"
      >
        <span class="flex items-center gap-8" aria-hidden="true">
          <span class="pip size-18 bg-scope-85-bg text-0-74rem">{{ SCOPE_META[band.scope].pip }}</span>
          <span class="font-mono text-meta tracking-kicker text-scope-78-text-2 uppercase">{{ SCOPE_META[band.scope].label }}</span>
        </span>
        <ul class="m-0 flex list-none flex-wrap gap-6 pl-0">
          <li
            v-for="d in band.items"
            :key="d.id"
            class="scope-specimen-legend-directive m-0"
          >
            <a class="body-link" :href="d.href"><code>{{ d.form }}</code></a>
          </li>
        </ul>
      </li>
    </ul>
  </div>
</template>
