<script setup lang="ts">
import { data as releases } from '../../../../data/pypi-releases.data'
</script>

<template>
  <section class="page-rv" aria-label="Install with release chronology">
    <p class="page-rv-lede">
      Take <em>Prose</em> to your own pages and make the next save <em>legible</em>.
    </p>

    <div class="page-rv-spread">
      <div class="page-rv-recto">
        <p class="kicker page-rv-kicker" data-variant="rule">Read on</p>
        <div class="page-rv-cmd" aria-label="Install command">
          <span class="page-rv-prompt" aria-hidden="true">$</span>
          <code>uv tool install prose-formatter</code>
        </div>
        <a class="page-rv-primary" href="/guide/quick-start">
          <span>Quick start</span>
          <span class="page-rv-arrow" aria-hidden="true">&rarr;</span>
        </a>
      </div>

      <div class="page-rv-verso" aria-label="Release chronology">
        <p class="kicker page-rv-kicker page-rv-kicker-right" data-variant="rule">On the shelf</p>
        <ol class="page-rv-list">
          <li
            v-for="(r, i) in releases"
            :key="r.version"
            :data-current="i === 0 || undefined"
          >
            <a :href="r.url" target="_blank" rel="noopener">
              <span class="page-rv-num">{{ r.version }}</span>
              <span class="page-rv-dot" aria-hidden="true">&middot;</span>
              <span class="page-rv-date">{{ r.date }}</span>
            </a>
          </li>
        </ol>
      </div>
    </div>
  </section>
</template>
