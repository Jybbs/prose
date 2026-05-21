<script setup lang="ts">
import { data as releases } from '../../../../data/pypi-releases.data'
</script>

<template>
  <section class="page-folio" aria-label="Install with release chronology">
    <div class="page-folio-head">
      <p class="kicker" data-variant="rule">Read on</p>

      <p class="page-folio-lede">
        Take <em>Prose</em> to your own pages and make the next save <em>legible</em>.
      </p>

      <div class="page-folio-line">
        <div class="page-folio-cmd" aria-label="Install command">
          <span class="page-folio-prompt" aria-hidden="true">$</span>
          <code>uv tool install prose-formatter</code>
        </div>
        <a class="page-folio-primary" href="/guide/quick-start">
          <span>Quick start</span>
          <span class="page-folio-arrow" aria-hidden="true">&rarr;</span>
        </a>
      </div>
    </div>

    <footer class="page-folio-colophon" aria-label="Release chronology">
      <p class="kicker page-folio-colophon-kicker">On the shelf</p>
      <ol class="page-folio-list">
        <li
          v-for="(r, i) in releases"
          :key="r.version"
          :data-current="i === 0 || undefined"
        >
          <span class="page-folio-index" aria-hidden="true">{{ String(i + 1).padStart(2, '0') }}</span>
          <a class="page-folio-version" :href="r.url" target="_blank" rel="noopener">
            <span class="page-folio-num">{{ r.version }}</span>
          </a>
          <span class="page-folio-leader" aria-hidden="true" />
          <span class="page-folio-date">{{ r.date }}</span>
        </li>
      </ol>
    </footer>
  </section>
</template>
