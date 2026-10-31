<script setup lang="ts">
import { computed, ref, useTemplateRef } from 'vue'

import { useKeyCard }     from '../../../lib/composables/use-key-card'
import { data as TOKENS } from '../../../lib/tokens/tokens.data'
import * as sources       from '../../../lib/tokens/sources'
import InlineProse        from '../base/InlineProse.vue'

const tabs = (Object.keys(sources.DOMAIN_LABELS) as sources.Domain[]).sort()

type View = 'all' | sources.Domain

const view = ref<View>('all')

const allGrouped    = computed(() => sources.groupByDomain(TOKENS))
const visibleGroups = computed(() => {
  if (view.value === 'all') return allGrouped.value
  return allGrouped.value.filter(([d]) => d === view.value)
})

const stage = useTemplateRef<HTMLElement>('stage')
const card  = useTemplateRef<HTMLElement>('card')

const { hide, keep, show, shown } = useKeyCard<sources.Token>(stage, card, 'a.az-index-key-btn, .az-index-section-head', 220)

// Splits the destination's last word, or the whole part set as code, from the
// words before it, so the last shares one unbreakable span with the leader.
const words = computed(() => {
  const destination = shown.value?.item.destination
  if (!destination) return null
  const text = destination.part ?? destination.title
  const cut  = destination.code ? 0 : text.lastIndexOf(' ') + 1
  return { ...destination, head: text.slice(0, cut), last: text.slice(cut) }
})
</script>

<template>
  <div ref="stage" class="az-index-stage">
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

    <div class="az-index-float-keys-wrap" :data-dim="shown !== null">
      <section
        v-for="[domain, tokens] in visibleGroups"
        :key="domain"
        class="az-index-section"
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
              :aria-current="shown?.item.key === token.key ? 'true' : undefined"
              @blur="hide"
              @focus="show($event.currentTarget as HTMLElement, token)"
              @mouseenter="show($event.currentTarget as HTMLElement, token)"
              @mouseleave="hide"
            >{{ token.key }}</a>
          </li>
        </ul>
      </section>
    </div>

    <aside
      v-if="shown && words"
      ref="card"
      class="az-index-detail"
      :data-domain="shown.item.domain"
      @mouseenter="keep"
      @mouseleave="hide"
    >
      <header class="az-index-detail-banner">
        <span class="az-index-detail-kicker">{{ sources.DOMAIN_LABELS[shown.item.domain] }}</span>
      </header>
      <div class="az-index-detail-body">
        <p class="az-index-detail-blurb"><InlineProse :nodes="shown.item.blurbNodes" /></p>
      </div>
      <footer class="az-index-detail-foot">
        <span id="az-index-destination" class="az-index-detail-dest">
          <template v-if="words.part">{{ words.title }} <span class="middle-dot" aria-hidden="true">·</span> </template>
          {{ words.head }}<span class="az-index-detail-tail"><code v-if="words.code" class="az-index-detail-code">{{ words.last }}</code><template v-else>{{ words.last }}</template><span class="az-index-detail-leader" aria-hidden="true"></span></span>
        </span>
        <a class="az-index-detail-href" :href="shown.item.href" aria-describedby="az-index-destination">
          <span class="nib-stroke">Read more <span aria-hidden="true">→</span></span>
        </a>
      </footer>
    </aside>
  </div>
</template>
