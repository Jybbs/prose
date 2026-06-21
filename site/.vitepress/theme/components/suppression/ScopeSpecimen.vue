<script setup lang="ts">
import { computed } from 'vue'

import { DECISIONS }               from './scope-decisions'
import { SCOPE_META, SCOPE_ORDER } from './scope-meta'

interface SpecimenLine {
  bracket : 'open' | 'close' | 'mid' | 'solo' | null
  scope   : 'file' | 'block' | 'line' | 'dict' | null
  text    : string
}

const DIRECTIVE_HREF: Record<string, string> = {
  'block-fmt'       : '/reference/suppression-directives#block-markers',
  'dict-keep'       : '/reference/suppression-directives#dict-literal-order-preservation',
  'file-off'        : '/reference/suppression-directives#file-level-suppression',
  'line-ignore'     : '/reference/suppression-directives#line-markers',
  'line-skip'       : '/reference/suppression-directives#line-markers',
  'line-skip-rules' : '/reference/suppression-directives#line-markers'
}

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
  { bracket : 'solo',  scope : 'line',  text : '    out = build(rows)  # fmt: skip' },
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

const legend = computed(() => SCOPE_ORDER.map(scope => ({
  items : DECISIONS.filter(d => d.scope === scope),
  scope
})))
</script>

<template>
  <div class="scope-specimen">
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
            <a class="body-link" :href="DIRECTIVE_HREF[d.id]"><code>{{ d.directive }}</code></a>
          </li>
        </ul>
      </li>
    </ul>
  </div>
</template>
