<script setup lang="ts">
import { computed, ref } from 'vue'

import { data as TOKENS } from '../../../lib/tokens/tokens.data'
import * as sources       from '../../../lib/tokens/sources'
import InlineProse        from '../base/InlineProse.vue'

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
  <div class="my-1-5rem">
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

    <div class="min-h-180 transition-opacity has-aria-current:opacity-55">
      <section
        v-for="[domain, tokens] in visibleGroups"
        :key="domain"
        class="not-first:mt-22"
        :data-domain="domain"
      >
        <header class="mb-8 flex items-baseline gap-12 border-b-1 border-b-divider pb-4">
          <span class="kicker text-domain-75-text-2">{{ sources.DOMAIN_LABELS[domain] }}</span>
          <span class="ml-auto font-mono text-meta tracking-meta text-text-3 uppercase">{{ tokens.length }} entries</span>
        </header>
        <ul class="m-0 list-none columns-2 gap-x-28 pl-0 column-rule-divider max-tablet:columns-1">
          <li v-for="token in tokens" :key="token.key" class="mt-0 mb-2 break-inside-avoid">
            <VDropdown
              theme="az-index"
              class="block"
              @apply-show="focus = token"
              @apply-hide="clearFocus(token)"
            >
              <a
                class="block w-full rounded-sm px-8 py-3 text-left font-mono text-xs wrap-anywhere whitespace-normal text-text-1 no-underline transition-colors hover:outline-none focus-visible:outline-none not-aria-current:hover:bg-domain/10 not-aria-current:hover:text-brand-2 not-aria-current:focus-visible:bg-domain/10 aria-current:bg-domain/20 aria-current:text-domain-85-text-1"
                :href="token.href"
                :aria-current="focus?.key === token.key ? 'true' : undefined"
              >{{ token.key }}</a>
              <template #popper>
                <aside class="flex flex-col overflow-hidden rounded-sm border-1 border-divider shadow-tooltip" :data-domain="token.domain">
                  <header class="border-b-1 border-b-divider bg-bg-alt px-12 py-4 text-right">
                    <span class="font-mono text-0-58rem tracking-display-wide text-domain-95-text-1 uppercase">{{ sources.DOMAIN_LABELS[token.domain] }}</span>
                  </header>
                  <div class="az-index-detail-body flex flex-col items-start gap-7 bg-panel-wash px-12 py-10">
                    <code class="text-1-15rem leading-1-1 wrap-anywhere text-text-1">{{ token.key }}</code>
                    <p class="text-sm leading-1-45 text-text-1"><InlineProse :nodes="token.blurbNodes" /></p>
                    <a class="font-mono text-0-58rem tracking-meta wrap-anywhere text-domain-70-text-2 transition-colors hover:text-domain-95-text-1" :href="token.href">&rarr; {{ token.href }}</a>
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
