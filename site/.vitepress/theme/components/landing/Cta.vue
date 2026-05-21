<script setup lang="ts">
import { computed, ref } from 'vue'

import { data as releases } from '../../../data/pypi-releases.data'
import { externalAttrs }    from '../../../lib/shared/links'

const current = releases[0]
const extras  = releases.slice(1)
const open    = ref(false)

const groupedByYear = computed(() =>
  Array.from(Map.groupBy(extras, r => r.year),
             ([year, items]) => ({ items, year }))
)
</script>

<template>
  <section class="landing-cta" :class="{ 'is-open': open }">
    <aside class="landing-cta-panel">
      <Transition name="landing-cta-swap" mode="out-in">
        <div v-if="!open" key="stamp" class="landing-cta-face">
          <a
            :href="current.url"
            v-bind="externalAttrs(current.url)"
            class="landing-cta-stamp"
            :aria-label="`Current release ${current.version}, ${current.month} ${current.year}`"
          >
            <span class="landing-cta-stamp-edge" aria-hidden="true"></span>
            <span class="landing-cta-stamp-month">{{ current.month }}</span>
            <span class="landing-cta-stamp-version">{{ current.version }}</span>
            <span class="landing-cta-stamp-year">'{{ current.yearShort }}</span>
          </a>
        </div>

        <div v-else key="open" class="landing-cta-face landing-cta-open">
          <div class="landing-cta-scroll">
            <div v-for="group in groupedByYear" :key="group.year" class="landing-cta-group">
              <p class="landing-cta-year">{{ group.year }}</p>
              <ol class="landing-cta-ledger">
                <li v-for="r in group.items" :key="r.version" class="landing-cta-ledger-row">
                  <a :href="r.url" v-bind="externalAttrs(r.url)">
                    <span class="landing-cta-ledger-version">{{ r.version }}</span>
                    <span class="landing-cta-ledger-leader" aria-hidden="true"></span>
                    <span class="landing-cta-ledger-month">{{ r.month }}</span>
                  </a>
                </li>
              </ol>
            </div>
          </div>
        </div>
      </Transition>

      <button
        type="button"
        class="landing-cta-toggle"
        :aria-expanded="open"
        :aria-label="open ? 'Back to current release' : 'Show previous releases'"
        @click="open = !open"
      >
        {{ open ? '←' : '+' }}
      </button>
    </aside>

    <div class="landing-cta-body">
      <p class="landing-cta-kicker kicker">Read on</p>
      <p class="landing-cta-lede">
        Take <em><span class="prose-mark">Prose</span></em> to your own pages and make the next save <em>legible</em>.
      </p>
      <div class="landing-cta-cmd" aria-label="Install command">
        <span class="landing-cta-prompt" aria-hidden="true">$</span>
        <code>uv tool install prose-formatter</code>
      </div>
      <a class="landing-cta-primary" href="/guide/quick-start">
        <span>Quick start</span>
        <span class="landing-cta-arrow" aria-hidden="true">→</span>
      </a>
    </div>
  </section>
</template>
