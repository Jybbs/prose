<script setup lang="ts">
import { computed }        from 'vue'

import { data as tools }   from '../../../../data/tools.data'
import { lookup }          from '../../../../lib/shared/lookup'

interface Column {
  role  : string
  slug ?: 'rust' | 'ruff' | 'uv'
  name ?: string
  mark ?: string
  note  : string
}

const columns: readonly Column[] = [
  { slug: 'rust',           role: 'Implementation',    note: 'Edits applied in a single rewriter pass.' },
  { slug: 'ruff',           role: 'Upstream parser',   note: 'Lossless lex and parse, leased intact.'   },
  { slug: 'uv',             role: 'Install path',      note: 'Wheel-first install for every user.'      },
  { name: 'mise', mark: '⌘', role: 'Tool versions',    note: 'Pinned toolchain, task runner glue.'      }
]

const entries = computed(() =>
  columns.map((c) => ({
    ...c,
    tool: c.slug ? lookup(tools.entries, c.slug, 'StackPrinterSlug column') : null
  }))
)
</script>

<template>
  <section class="stack-slug">
    <header class="stack-slug-head">
      <p class="stack-slug-kicker">— The Stack —</p>
      <h2 class="stack-slug-heading">Standing on <em>giants</em>.</h2>
    </header>

    <div class="stack-slug-row">
      <component
        :is="entry.tool ? 'a' : 'div'"
        v-for="entry in entries"
        :key="entry.slug ?? entry.name"
        class="stack-slug-col"
        :href="entry.tool?.href"
        :target="entry.tool ? '_blank' : null"
        :rel="entry.tool ? 'noopener' : null"
      >
        <span class="stack-slug-medallion" aria-hidden="true">
          <svg
            v-if="entry.tool"
            class="stack-slug-glyph"
            :viewBox="entry.tool.icon.viewBox"
            v-html="entry.tool.icon.body"
          />
          <span v-else class="stack-slug-mark">{{ entry.mark }}</span>
        </span>
        <span class="stack-slug-name">{{ entry.tool?.name ?? entry.name }}</span>
        <span class="stack-slug-role">{{ entry.role }}</span>
        <span class="stack-slug-rule" aria-hidden="true" />
        <span class="stack-slug-note">{{ entry.note }}</span>
      </component>
    </div>

    <p class="stack-slug-foot">
      <span class="stack-slug-foot-label">also</span>
      <code>pre-commit</code>
      <span class="stack-slug-foot-dot" aria-hidden="true">·</span>
      <code>VitePress</code>
      <span class="stack-slug-foot-dot" aria-hidden="true">·</span>
      <code>GitHub Actions</code>
    </p>
  </section>
</template>
