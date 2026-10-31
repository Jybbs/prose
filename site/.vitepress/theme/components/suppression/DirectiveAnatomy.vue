<script setup lang="ts">
import { data as directives }     from '../../../lib/suppression/directives.data'
import { useTabSelect }           from '../../../lib/composables/use-tab-select'
import type { PartRole }          from '../../../lib/suppression/directive-parts'
import { SCOPE_META, scopeBands } from '../../../lib/suppression/scopes'
import InlineProse                from '../base/InlineProse.vue'

const ROLE_LABEL: Record<PartRole, string> = {
  action    : 'action',
  comment   : 'comment',
  namespace : 'namespace',
  payload   : 'rule list'
}

const bands = scopeBands(directives)

const { active: focused, selected: focusId } = useTabSelect(directives, d => d.id)
focusId.value = 'prose-ignore-rules'

function classifyLine(line: string): 'directive' | 'comment' | 'code' {
  const trimmed = line.trim()
  if (trimmed.includes(' fmt:') || trimmed.includes(' yapf:') || trimmed.includes(' prose:')) {
    return 'directive'
  }
  if (trimmed.startsWith('#')) return 'comment'
  return 'code'
}
</script>

<template>
  <div class="directive-anatomy overflow-hidden panel bg-panel-wash">
    <div class="flex flex-col gap-20 px-20 pt-18 pb-22" role="tablist" aria-label="Directive index">
      <section
        v-for="band in bands"
        :key="band.scope"
        class="directive-anatomy-band flex flex-col gap-8"
        :data-scope="band.scope"
      >
        <header class="directive-anatomy-band-head flex items-center gap-9">
          <span class="pip size-22 shrink-0 bg-scope-80-bg text-0-92rem leading-1" aria-hidden="true">{{ SCOPE_META[band.scope].pip }}</span>
          <span class="band-name">{{ SCOPE_META[band.scope].label }}</span>
          <span class="hairline" aria-hidden="true"></span>
        </header>
        <div class="flex flex-wrap gap-6">
          <button
            v-for="d in band.items"
            :key="d.id"
            type="button"
            role="tab"
            class="directive-anatomy-thumb group inline-flex items-center rounded-3 border-1 border-divider bg-bg px-9 pt-2 pb-3 transition-border-color-and-background chosen:border-scope-60-divider chosen:bg-scope-10-bg"
            :data-scope="d.scope"
            :data-active="focusId === d.id"
            :aria-selected="focusId === d.id"
            @click="focusId = d.id"
            @mouseenter="focusId = d.id"
          >
            <span class="border-none font-mono text-xxs leading-1-5 text-text-2 group-chosen:text-text-1">{{ d.form }}</span>
          </button>
        </div>
      </section>
    </div>

    <div class="directive-anatomy-plate" :data-scope="focused.scope">
      <div class="flex flex-wrap items-start gap-0">
        <span
          v-for="(part, i) in focused.parts"
          :key="i"
          class="directive-anatomy-part-col group"
          :data-role="part.role"
        >
          <span class="directive-anatomy-part group-role-action:font-600">{{ part.text }}</span>
          <span class="my-2 h-24 w-1 bg-part-or-text-3-50-divider" aria-hidden="true"></span>
          <span class="font-display text-0-92rem text-part-or-text-2-75-text-2 italic">{{ ROLE_LABEL[part.role] }}</span>
        </span>
      </div>

      <div class="flex flex-col gap-4">
        <p class="directive-anatomy-effect max-w-72ch text-0-98rem text-text-2 italic"><InlineProse :nodes="focused.effectNodes" /></p>
        <pre class="overflow-x-auto rounded-sm border-1 border-divider bg-bg px-14 py-12 text-code leading-1-55 text-text-2"><code><span
          v-for="(line, j) in focused.example.split('\n')"
          :key="j"
          class="block line-code:text-text-1 line-comment:text-text-3 line-directive:font-500 line-directive:text-apricot-75-text-1"
          :data-line-kind="classifyLine(line)"
        >{{ line }}
</span></code></pre>
      </div>
    </div>
  </div>
</template>
