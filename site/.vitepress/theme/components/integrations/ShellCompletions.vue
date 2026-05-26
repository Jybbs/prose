<script setup lang="ts">
import Tool from '../base/Tool.vue'

import { data as shells } from '../../../data/shell-completions.data'
import { useTabSelect }   from '../../../lib/composables/use-tab-select'

const { selected: activeSlug, active } = useTabSelect(shells, s => s.slug)
</script>

<template>
  <div class="shell-card">
    <div class="shell-card-stack" role="tablist" aria-label="Shell completion targets">
      <button
        v-for="(shell, i) in shells"
        :key="shell.slug"
        type="button"
        role="tab"
        class="shell-card-tab"
        :class="{ 'is-active': shell.slug === activeSlug }"
        :aria-selected="shell.slug === activeSlug"
        :style="{ '--idx': i }"
        @click="activeSlug = shell.slug"
      >
        <span class="shell-card-tab-mark" aria-hidden="true"><Tool :slug="shell.slug" bare /></span>
        <em class="shell-card-tab-name">{{ shell.name }}</em>
      </button>

      <article class="shell-card-modal">
        <header class="shell-card-modal-head">
          <span class="shell-card-modal-mark" aria-hidden="true"><Tool :slug="active.slug" bare /></span>
          <span class="kicker">{{ active.caption }}</span>
        </header>
        <div class="shell-card-modal-body">
          <div class="shell-card-modal-code" v-html="active.codeHtml"></div>
          <p class="shell-card-modal-note" v-html="active.noteHtml"></p>
        </div>
      </article>
    </div>
  </div>
</template>
