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
  <div ref="stage" class="az-index-stage relative my-1-5rem">
    <nav class="mb-18 flex flex-wrap items-end gap-x-10 gap-y-0" role="tablist" aria-label="Domain tabs">
      <button
        type="button"
        role="tab"
        :aria-selected="view === 'all'"
        class="relative inline-flex items-center border-none px-3 pt-4 pb-10 font-mono text-0-56rem tracking-meta uppercase transition-colors after:absolute after:inset-x-0 after:bottom-0 after:transition-background-and-height text-text-2 after:h-2 after:bg-text-3/65 not-aria-selected:hover:text-text-1 not-aria-selected:hover:after:bg-text-2/80 aria-selected:text-text-1 aria-selected:after:h-6 aria-selected:after:bg-text-1"
        @click="view = 'all'"
      >All</button>
      <button
        v-for="d in tabs"
        :key="d"
        type="button"
        role="tab"
        :aria-selected="view === d"
        class="relative inline-flex items-center border-none px-3 pt-4 pb-10 font-mono text-0-56rem tracking-meta uppercase transition-colors after:absolute after:inset-x-0 after:bottom-0 after:transition-background-and-height text-text-2 after:h-2 after:bg-domain/65 not-aria-selected:hover:text-text-1 not-aria-selected:hover:after:bg-domain/80 aria-selected:text-text-1 aria-selected:after:h-6 aria-selected:after:bg-domain"
        :data-domain="d"
        @click="view = d"
      >{{ sources.DOMAIN_LABELS[d] }}</button>
    </nav>

    <div class="az-index-float-keys-wrap min-h-180" :data-dim="shown !== null">
      <section
        v-for="[domain, tokens] in visibleGroups"
        :key="domain"
        class="not-first:mt-22"
      >
        <header class="az-index-section-head mb-8 flex items-baseline gap-12 border-b-1 border-b-divider pb-4">
          <span class="kicker text-text-2">{{ sources.DOMAIN_LABELS[domain] }}</span>
          <span class="ml-auto font-mono text-meta tracking-meta text-text-2 uppercase">{{ tokens.length }} entries</span>
        </header>
        <ul class="m-0 list-none columns-2 gap-x-28 pl-0 column-rule-divider max-tablet:columns-1">
          <li v-for="token in tokens" :key="token.key" class="mt-0 mb-2 break-inside-avoid" :data-domain="domain">
            <a
              class="az-index-key-btn block w-full px-8 py-3 text-left font-mono text-xs wrap-anywhere whitespace-normal text-text-1 no-underline outline-none not-aria-current:hover:bg-text-1/10 not-aria-current:hover:text-brand-2 not-aria-current:focus-visible:bg-text-1/10 aria-current:bg-domain-band"
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
      class="az-index-detail absolute top-0 left-0 z-10 flex flex-col rounded-base border-1 border-divider bg-bg shadow-tooltip data-snap:transition-none data-[side=bottom]:rounded-t-none data-[side=left]:rounded-tr-none data-[side=right]:rounded-tl-none motion-reduce:transition-none"
      :data-domain="shown.item.domain"
      @mouseenter="keep"
      @mouseleave="hide"
    >
      <header class="az-index-detail-banner relative -mx-1 -mt-1 mb-0 box-border flex items-center rounded-b-none px-12 py-0">
        <span class="font-mono text-meta tracking-kicker text-text-1 uppercase">{{ sources.DOMAIN_LABELS[shown.item.domain] }}</span>
      </header>
      <div class="az-index-detail-body">
        <p class="m-0 font-base text-sm leading-1-5 text-text-1"><InlineProse :nodes="shown.item.blurbNodes" /></p>
      </div>
      <footer class="flex items-baseline-last gap-8 border-t-1 border-t-divider px-16 pt-7 pb-8">
        <span id="az-index-destination" class="min-w-0 flex-1 overflow-clip font-base text-xs leading-1-4 text-text-2">
          <template v-if="words.part">{{ words.title }} <span class="middle-dot" aria-hidden="true">·</span> </template>
          {{ words.head }}<span class="whitespace-nowrap"><code v-if="words.code" class="az-index-detail-code">{{ words.last }}</code><template v-else>{{ words.last }}</template><span class="az-index-detail-leader inline-block w-full align-baseline" aria-hidden="true"></span></span>
        </span>
        <a class="flex-none font-mono text-label tracking-meta whitespace-nowrap text-text-1 uppercase no-underline" :href="shown.item.href" aria-describedby="az-index-destination">
          <span class="nib-stroke pb-1">Read more <span aria-hidden="true">→</span></span>
        </a>
      </footer>
    </aside>
  </div>
</template>
