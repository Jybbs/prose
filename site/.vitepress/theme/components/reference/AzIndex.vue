<script setup lang="ts">
import { computed, ref } from 'vue'

import { data as TOKENS } from '../../../data/tokens.data'
import * as sources       from '../../../lib/tokens/sources'

const tabs = (Object.keys(sources.DOMAIN_LABELS) as sources.Domain[]).sort()

type View = 'all' | sources.Domain

const view  = ref<View>('all')
const focus = ref<sources.Token | null>(null)

const allGrouped    = computed(() => sources.groupByDomain(TOKENS))
const visibleGroups = computed(() => {
  if (view.value === 'all') return allGrouped.value
  return allGrouped.value.filter(([d]) => d === view.value)
})

function clearFocus(token: sources.Token): void {
  if (focus.value?.key === token.key) focus.value = null
}
</script>

<template>
  <div class="az-index-stage">
    <nav class="az-index-tabs" role="tablist" aria-label="Domain tabs">
      <button
        type="button"
        role="tab"
        :aria-selected="view === 'all'"
        class="az-index-tab"
        :class="{ 'is-active': view === 'all' }"
        @click="view = 'all'"
      >All</button>
      <button
        v-for="d in tabs"
        :key="d"
        type="button"
        role="tab"
        :aria-selected="view === d"
        class="az-index-tab"
        :class="{ 'is-active': view === d }"
        :data-domain="d"
        @click="view = d"
      >{{ sources.DOMAIN_LABELS[d] }}</button>
    </nav>

    <div class="az-index-float-keys-wrap" :data-dim="focus !== null">
      <section
        v-for="[domain, tokens] in visibleGroups"
        :key="domain"
        class="az-index-section"
        :data-domain="domain"
      >
        <header class="az-index-section-head">
          <span class="kicker az-index-section-folio">{{ sources.DOMAIN_LABELS[domain] }}</span>
          <span class="az-index-section-count">{{ tokens.length }} entries</span>
        </header>
        <ul class="az-index-keys">
          <li v-for="token in tokens" :key="token.key" class="az-index-key" :data-domain="domain">
            <VDropdown
              theme="az-index"
              @apply-show="focus = token"
              @apply-hide="clearFocus(token)"
            >
              <a
                class="az-index-key-btn"
                :href="token.href"
                :aria-current="focus?.key === token.key ? 'true' : undefined"
              >{{ token.key }}</a>
              <template #popper>
                <aside class="az-index-detail" :data-domain="token.domain">
                  <header class="az-index-detail-banner">
                    <span class="az-index-detail-kicker">{{ sources.DOMAIN_LABELS[token.domain] }}</span>
                  </header>
                  <div class="az-index-detail-body">
                    <code class="az-index-detail-key">{{ token.key }}</code>
                    <p class="az-index-detail-blurb" v-html="token.blurbHtml" />
                    <a class="az-index-detail-href" :href="token.href">&rarr; {{ token.href }}</a>
                  </div>
                </aside>
              </template>
            </VDropdown>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>
