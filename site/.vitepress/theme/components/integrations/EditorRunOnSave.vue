<script setup lang="ts">
import Tool from '../base/Tool.vue'

import { data as editors } from '../../../data/editor-configs.data'
import { useTabSelect }    from '../../../lib/composables/use-tab-select'
import { formatFolio }     from '../../../lib/shared/numerals'

const { selected: activeSlug } = useTabSelect(editors, e => e.slug)
</script>

<template>
  <div class="editor-card">
    <aside class="editor-card-index">
      <span class="kicker editor-card-edition">Editors &middot; {{ formatFolio(editors.length) }}</span>
      <ul class="editor-card-list" role="tablist">
        <li v-for="(editor, i) in editors" :key="editor.slug">
          <button
            type="button"
            role="tab"
            class="editor-card-row"
            :class="{ 'is-active' : editor.slug === activeSlug }"
            :aria-selected="editor.slug === activeSlug"
            @click="activeSlug = editor.slug"
          >
            <span class="folio">№ {{ formatFolio(i + 1) }}</span>
            <span class="editor-card-row-mark" aria-hidden="true"><Tool :slug="editor.slug" bare /></span>
            <em class="editor-card-row-name">{{ editor.name }}</em>
            <span class="editor-card-row-leader" aria-hidden="true"></span>
          </button>
        </li>
      </ul>
    </aside>

    <div class="editor-card-faces" aria-live="polite">
      <section
        v-for="editor in editors"
        :key="editor.slug"
        v-show="editor.slug === activeSlug"
        class="editor-card-face"
      >
        <header class="editor-card-face-head">
          <span class="kicker">{{ editor.caption }}</span>
          <span class="editor-card-face-target">{{ editor.target }}</span>
        </header>
        <div class="editor-card-face-code" v-html="editor.codeHtml" />
      </section>
    </div>
  </div>
</template>
