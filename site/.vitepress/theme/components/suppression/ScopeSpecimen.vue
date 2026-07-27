<script setup lang="ts">
import { data as directives, type Directive }    from '../../../lib/suppression/directives.data'
import { directiveHref, SCOPE_META, scopeBands } from '../../../lib/suppression/scopes'
import type { ScopeKey }                         from '../../../lib/suppression/scopes'

interface SpecimenLine {
  bracket : 'open' | 'close' | 'mid' | 'solo' | null
  scope   : ScopeKey | null
  text    : string
}

const LEGEND_IDS = [
  'prose-off', 'fmt-off', 'fmt-skip', 'prose-skip-rules', 'prose-ignore-rules', 'prose-keep'
]

const lines: SpecimenLine[] = [
  { bracket : 'open',  scope : 'file',  text : '# prose: off' },
  { bracket : 'mid',   scope : 'file',  text : '' },
  { bracket : 'mid',   scope : 'file',  text : 'from collections import OrderedDict' },
  { bracket : 'mid',   scope : 'file',  text : '' },
  { bracket : 'mid',   scope : 'file',  text : 'def render(rows):' },
  { bracket : 'open',  scope : 'block', text : '    # fmt: off' },
  { bracket : 'mid',   scope : 'block', text : '    grid = [[1, 0, 0, 0],' },
  { bracket : 'mid',   scope : 'block', text : '            [0, 1, 0, 0],' },
  { bracket : 'mid',   scope : 'block', text : '            [0, 0, 1, 0],' },
  { bracket : 'mid',   scope : 'block', text : '            [0, 0, 0, 1]]' },
  { bracket : 'close', scope : 'block', text : '    # fmt: on' },
  { bracket : 'mid',   scope : 'file',  text : '' },
  { bracket : 'open',  scope : 'line',  text : '    out = build(' },
  { bracket : 'mid',   scope : 'line',  text : '        rows,' },
  { bracket : 'close', scope : 'line',  text : '    )  # fmt: skip' },
  { bracket : 'solo',  scope : 'line',  text : '    log(out)  # prose: ignore[<rule>]' },
  { bracket : 'mid',   scope : 'file',  text : '' },
  { bracket : 'open',  scope : 'dict',  text : '    STAGES = {  # prose: keep' },
  { bracket : 'mid',   scope : 'dict',  text : '        "fetch"    : fetch,' },
  { bracket : 'mid',   scope : 'dict',  text : '        "parse"    : parse,' },
  { bracket : 'mid',   scope : 'dict',  text : '        "validate" : validate,' },
  { bracket : 'mid',   scope : 'dict',  text : '        "render"   : render,' },
  { bracket : 'close', scope : 'dict',  text : '    }' },
  { bracket : 'close', scope : 'file',  text : '' }
]

function legendDirective(id: string): Directive {
  const directive = directives.find(d => d.id === id)
  if (directive === undefined) {
    throw new Error(`scope specimen: no directive with id "${id}"`)
  }
  return directive
}

const entries = LEGEND_IDS.map(id => {
  const directive = legendDirective(id)
  return {
    form  : directive.pairRole === 'opens' && directive.pairId !== undefined
      ? `${directive.form} … ${legendDirective(directive.pairId).form}`
      : directive.form,
    href  : directiveHref(directive.scope),
    id    : id,
    scope : directive.scope
  }
})

const legend = scopeBands(entries)
</script>

<template>
  <div class="scope-specimen panel">
    <pre class="scope-specimen-source"><code><span
      v-for="(line, idx) in lines"
      :key="idx"
      class="scope-specimen-line"
      :data-scope="line.scope"
      :data-bracket="line.bracket"
    ><span class="scope-specimen-gutter" aria-hidden="true">{{ String(idx + 1).padStart(2, ' ') }}</span><span class="scope-specimen-bracket" aria-hidden="true"></span><span class="scope-specimen-code">{{ line.text || ' ' }}</span></span></code></pre>

    <ul class="scope-specimen-legend">
      <li
        v-for="band in legend"
        :key="band.scope"
        class="scope-specimen-legend-row"
        :data-scope="band.scope"
      >
        <span class="scope-specimen-legend-marker" aria-hidden="true">
          <span class="scope-specimen-legend-pip">{{ SCOPE_META[band.scope].pip }}</span>
          <span class="scope-specimen-legend-name">{{ SCOPE_META[band.scope].label }}</span>
        </span>
        <ul class="scope-specimen-legend-directives">
          <li
            v-for="d in band.items"
            :key="d.id"
            class="scope-specimen-legend-directive"
          >
            <a class="body-link" :href="d.href"><code>{{ d.form }}</code></a>
          </li>
        </ul>
      </li>
    </ul>
  </div>
</template>
