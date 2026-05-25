<script setup lang="ts">
import { computed, ref } from 'vue'

import { SCOPE_META } from './scope-decisions'

import {
  data as directives,
  type Directive,
  type PartRole,
  type Scope
} from '../../../data/directives.data'

const ROLE_LABEL: Record<PartRole, string> = {
  action    : 'action',
  comment   : 'comment',
  namespace : 'namespace',
  payload   : 'rule list'
}

const DIRECTIVE_SCOPE_ORDER: Scope[] = ['file', 'block', 'line']

const bands = computed(() => {
  return DIRECTIVE_SCOPE_ORDER.map(scope => ({
    scope,
    items : directives.filter(d => d.scope === scope)
  }))
})

const focusId = ref<string>('prose-ignore-rules')

const focused = computed<Directive>(() =>
  directives.find(d => d.id === focusId.value) ?? directives[0]
)

function classifyLine(line: string): 'directive' | 'comment' | 'code' {
  const trimmed = line.trim()
  if (trimmed.includes(' fmt:') || trimmed.includes(' yapf:') || trimmed.includes(' prose:')) return 'directive'
  if (trimmed.startsWith('#')) return 'comment'
  return 'code'
}
</script>

<template>
  <div class="directive-anatomy">
    <div class="directive-anatomy-index" role="tablist" aria-label="Directive index">
      <section
        v-for="band in bands"
        :key="band.scope"
        class="directive-anatomy-band"
        :data-scope="band.scope"
      >
        <header class="directive-anatomy-band-head">
          <span class="directive-anatomy-band-badge" aria-hidden="true">{{ SCOPE_META[band.scope].pip }}</span>
          <span class="directive-anatomy-band-name">{{ SCOPE_META[band.scope].label }}</span>
          <span class="directive-anatomy-band-rule" aria-hidden="true"></span>
        </header>
        <div class="directive-anatomy-band-cells">
          <button
            v-for="d in band.items"
            :key="d.id"
            type="button"
            role="tab"
            class="directive-anatomy-thumb"
            :data-scope="d.scope"
            :data-active="focusId === d.id"
            :aria-selected="focusId === d.id"
            @click="focusId = d.id"
            @mouseenter="focusId = d.id"
          >
            <span class="directive-anatomy-thumb-form">{{ d.form }}</span>
          </button>
        </div>
      </section>
    </div>

    <div class="directive-anatomy-plate" :data-scope="focused.scope">
      <div class="directive-anatomy-specimen">
        <span
          v-for="(part, i) in focused.parts"
          :key="i"
          class="directive-anatomy-part-col"
          :data-role="part.role"
        >
          <span class="directive-anatomy-part">{{ part.text }}</span>
          <span class="directive-anatomy-tick" aria-hidden="true"></span>
          <span class="directive-anatomy-label">{{ ROLE_LABEL[part.role] }}</span>
        </span>
      </div>

      <div class="directive-anatomy-info">
        <p class="directive-anatomy-effect" v-html="focused.effectHtml"></p>
        <pre class="directive-anatomy-pre"><code><span
          v-for="(line, j) in focused.example.split('\n')"
          :key="j"
          class="directive-anatomy-line"
          :data-line-kind="classifyLine(line)"
        >{{ line }}
</span></code></pre>
      </div>
    </div>
  </div>
</template>
