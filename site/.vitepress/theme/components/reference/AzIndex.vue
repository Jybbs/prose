<script setup lang="ts">
import { useTimeoutFn }  from '@vueuse/core'
import { computed, ref } from 'vue'

import { data as TOKENS } from '../../../data/tokens.data'
import * as sources       from '../../../lib/tokens/sources'

const tabs = (Object.keys(sources.DOMAIN_LABELS) as sources.Domain[]).sort()

type View = 'all' | sources.Domain

const view  = ref<View>('all')
const focus = ref<sources.Token | null>(null)
const popX  = ref(0)
const popY  = ref(0)

const { start: scheduleClear, stop: cancelClear } = useTimeoutFn(
  () => { focus.value = null }, 220, { immediate: false }
)

const allGrouped    = computed(() => sources.groupByDomain(sources.sortedTokens(TOKENS, 'key')))
const visibleGroups = computed(() => {
  if (view.value === 'all') return allGrouped.value
  return allGrouped.value.filter(([d]) => d === view.value)
})

function onEnter(token: sources.Token, event: MouseEvent | FocusEvent): void {
  cancelClear()
  focus.value = token
  const rect  = (event.currentTarget as HTMLElement).getBoundingClientRect()
  popX.value  = rect.right + 14
  popY.value  = rect.top
}
</script>

<template>
  <div class="az-index-stage">
    <nav class="az-index-tabs" role="tablist" aria-label="sources.Domain tabs">
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

    <div class="az-index-float-wrap">
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
              <a
                class="az-index-key-btn"
                :href="token.href"
                :aria-current="focus?.key === token.key ? 'true' : undefined"
                @mouseenter="onEnter(token, $event)"
                @mouseleave="scheduleClear"
                @focus="onEnter(token, $event)"
                @blur="scheduleClear"
              >{{ token.key }}</a>
            </li>
          </ul>
        </section>
      </div>

      <div
        v-if="focus"
        class="az-index-float-pop"
        :style="{ left: popX + 'px', top: popY + 'px' }"
        @mouseenter="cancelClear"
        @mouseleave="scheduleClear"
      >
        <aside class="az-index-detail" :data-domain="focus.domain">
          <header class="az-index-detail-banner">
            <span class="az-index-detail-kicker">{{ sources.DOMAIN_LABELS[focus.domain] }}</span>
          </header>
          <div class="az-index-detail-body">
            <code class="az-index-detail-key">{{ focus.key }}</code>
            <p class="az-index-detail-blurb" v-html="focus.blurbHtml" />
            <a class="az-index-detail-href" :href="focus.href">&rarr; {{ focus.href }}</a>
          </div>
        </aside>
      </div>
    </div>
  </div>
</template>
