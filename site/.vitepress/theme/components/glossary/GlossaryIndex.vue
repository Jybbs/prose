<script setup lang="ts">
import { computed } from 'vue'

import { data as glossary, type RenderedGlossaryEntry } from '../../../data/glossary.data'

const grouped = computed<[string, RenderedGlossaryEntry[]][]>(() => {
  const entries = Object.values(glossary.entries).sort((a, b) =>
    a.slug.localeCompare(b.slug, 'en', { sensitivity: 'base' }))
  const map = new Map<string, RenderedGlossaryEntry[]>()
  for (const entry of entries) {
    const bucket = map.get(entry.initial) ?? []
    bucket.push(entry)
    map.set(entry.initial, bucket)
  }
  return [...map.entries()].sort(([a], [b]) => a.localeCompare(b))
})
</script>

<template>
  <div class="glossary-index">
    <section v-for="[initial, entries] in grouped" :key="initial" class="glossary-section">
      <h2 :id="initial.toLowerCase()" class="glossary-initial">{{ initial }}</h2>
      <dl class="glossary-entries">
        <template v-for="entry in entries" :key="entry.slug">
          <dt class="glossary-term">
            <a v-if="entry.href" :href="entry.href" class="glossary-term-link"><code>{{ entry.slug }}</code></a>
            <code v-else>{{ entry.slug }}</code>
            <span v-if="entry.aliases.length > 0" class="glossary-aliases">
              <template v-for="(alias, idx) in entry.aliases" :key="alias">
                <code>{{ alias }}</code><span v-if="idx < entry.aliases.length - 1">, </span>
              </template>
            </span>
          </dt>
          <dd class="glossary-definition" v-html="entry.definitionHtml" />
        </template>
      </dl>
    </section>
  </div>
</template>

