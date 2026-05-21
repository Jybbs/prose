<script setup lang="ts">
import { data as releases } from '../../../../data/pypi-releases.data'
</script>

<template>
  <section class="page-marquee" aria-label="Install with release chronology">
    <div class="page-marquee-bar">
      <p class="kicker page-marquee-kicker" data-variant="rule">Read on</p>

      <div class="page-marquee-cmd" aria-label="Install command">
        <span class="page-marquee-prompt" aria-hidden="true">$</span>
        <code>uv tool install prose-formatter</code>
      </div>

      <a class="page-marquee-primary" href="/guide/quick-start">
        <span>Quick start</span>
        <span class="page-marquee-arrow" aria-hidden="true">&rarr;</span>
      </a>
    </div>

    <p class="page-marquee-lede">
      Take <em>Prose</em> to your own pages and make the next save <em>legible</em>.
    </p>

    <div class="page-marquee-strip" aria-label="Release chronology">
      <p class="kicker page-marquee-strip-kicker">On the shelf</p>
      <ol class="page-marquee-list">
        <li
          v-for="(r, i) in releases"
          :key="r.version"
          :data-current="i === 0 || undefined"
        >
          <a :href="r.url" target="_blank" rel="noopener">
            <span class="page-marquee-num">{{ r.version }}</span>
            <span class="page-marquee-date">{{ r.date }}</span>
          </a>
        </li>
      </ol>
    </div>
  </section>
</template>
