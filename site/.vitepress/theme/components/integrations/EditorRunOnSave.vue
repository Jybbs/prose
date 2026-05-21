<script setup lang="ts">
import Tool from '../base/Tool.vue'

import { data as editors } from '../../../data/editor-configs.data'
import { useTabSelect }    from '../../../lib/shared/use-tab-select'

const { active, selected: activeSlug } = useTabSelect(editors, e => e.slug)
</script>

<template>
  <div class="editor-deck">
    <div class="editor-deck-spines" role="tablist">
      <button
        v-for="(editor, idx) in editors"
        :key="editor.slug"
        type="button"
        role="tab"
        class="editor-deck-spine"
        :class="{ 'is-active': editor.slug === activeSlug }"
        :aria-selected="editor.slug === activeSlug"
        @click="activeSlug = editor.slug"
      >
        <span class="editor-deck-spine-num">{{ String(idx + 1).padStart(2, '0') }}</span>
        <span class="editor-deck-spine-mark" aria-hidden="true"><Tool :slug="editor.slug" bare /></span>
        <span class="editor-deck-spine-name">{{ editor.name }}</span>
        <span class="editor-deck-spine-target">{{ editor.target }}</span>
      </button>
    </div>
    <div class="editor-deck-face">
      <div class="editor-deck-face-head">
        <span class="editor-deck-face-mark" aria-hidden="true"><Tool :slug="active.slug" bare /></span>
        <h3 class="editor-deck-face-name">{{ active.name }}</h3>
      </div>
      <span class="editor-deck-face-caption">{{ active.target }}</span>
      <div :key="active.slug" class="editor-deck-face-body" v-html="active.codeHtml" />
    </div>
  </div>
</template>
