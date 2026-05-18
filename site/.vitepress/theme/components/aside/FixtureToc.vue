<script setup lang="ts">
import './fixture-toc.css'

import { inBrowser, onContentUpdated } from 'vitepress'
import { ref }                         from 'vue'

import Kicker from '../base/Kicker.vue'

import { useIsRulePage } from '../../../lib/shared/route'

interface Anchor {
  href : string
  title: string
}

const isRulePage = useIsRulePage()
const anchors    = ref<Anchor[]>([])

function collect() {
  if (!isRulePage.value || !inBrowser) {
    anchors.value = []
    return
  }
  const summaries = Array.from(document.querySelectorAll('.disclosure-fixture .disclosure-summary .disclosure-title'))
  const seen      = new Set<string>()
  const found: Anchor[] = []
  for (const node of summaries) {
    const title = node.textContent?.trim() ?? ''
    if (!title || seen.has(title)) continue
    seen.add(title)
    const disclosure = node.closest('details')
    let id           = disclosure?.id
    if (!id && disclosure) {
      id            = `fixture-${found.length + 1}`
      disclosure.id = id
    }
    if (id) found.push({ href: `#${id}`, title })
  }
  anchors.value = found
}

onContentUpdated(collect)
</script>

<template>
  <div v-if="anchors.length" class="fixture-toc">
    <Kicker class="fixture-toc-kicker">Examples</Kicker>
    <ul class="fixture-toc-list">
      <li v-for="anchor in anchors" :key="anchor.href">
        <a :href="anchor.href">{{ anchor.title }}</a>
      </li>
    </ul>
  </div>
</template>
