<script setup lang="ts">
import { computed, ref } from 'vue'

import { data as releases } from '../../../data/pypi-releases.data'

interface Release {
  date    : string
  url     : string
  version : string
}

const current = releases[0]
const extras  = computed(() => releases.slice(1))
const open    = ref(false)

function monthLabel(date: string): string {
  const months = ['JAN', 'FEB', 'MAR', 'APR', 'MAY', 'JUN', 'JUL', 'AUG', 'SEP', 'OCT', 'NOV', 'DEC']
  const m      = Number.parseInt(date.split('-')[1], 10) - 1
  return months[m] ?? '—'
}
function yearOf(date: string): string {
  return date.slice(2, 4)
}
function fullYearOf(date: string): string {
  return date.slice(0, 4)
}

const groupedByYear = computed(() => {
  const groups: { year: string; items: Release[] }[] = []
  for (const r of extras.value) {
    const y    = fullYearOf(r.date)
    const last = groups[groups.length - 1]
    if (last && last.year === y) last.items.push(r as Release)
    else groups.push({ year: y, items: [r as Release] })
  }
  return groups
})
</script>

<template>
  <section class="landing-cta" :class="{ 'is-open': open }">
    <aside class="landing-cta-panel">
      <Transition name="landing-cta-swap" mode="out-in">
        <div v-if="!open" key="stamp" class="landing-cta-face">
          <a
            :href="current.url"
            target="_blank"
            rel="noopener"
            class="landing-cta-stamp"
            :aria-label="`Current release ${current.version}, ${monthLabel(current.date)} ${current.date.slice(0, 4)}`"
          >
            <span class="landing-cta-stamp-edge" aria-hidden="true"></span>
            <span class="landing-cta-stamp-month">{{ monthLabel(current.date) }}</span>
            <span class="landing-cta-stamp-version">{{ current.version }}</span>
            <span class="landing-cta-stamp-year">'{{ yearOf(current.date) }}</span>
          </a>
        </div>

        <div v-else key="open" class="landing-cta-face landing-cta-open">
          <div class="landing-cta-scroll">
            <div v-for="group in groupedByYear" :key="group.year" class="landing-cta-group">
              <p class="landing-cta-year">{{ group.year }}</p>
              <ol class="landing-cta-ledger">
                <li v-for="r in group.items" :key="r.version" class="landing-cta-ledger-row">
                  <a :href="r.url" target="_blank" rel="noopener">
                    <span class="landing-cta-ledger-version">{{ r.version }}</span>
                    <span class="landing-cta-ledger-leader" aria-hidden="true"></span>
                    <span class="landing-cta-ledger-month">{{ monthLabel(r.date) }}</span>
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
        <span class="landing-cta-toggle-glyph" aria-hidden="true">{{ open ? '←' : '+' }}</span>
      </button>
    </aside>

    <div class="landing-cta-body">
      <p class="landing-cta-kicker">Read on</p>
      <p class="landing-cta-lede">
        Take <em>Prose</em> to your own pages and make the next save <em>legible</em>.
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
