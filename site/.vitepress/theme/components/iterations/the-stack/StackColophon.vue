<script setup lang="ts">
import { computed }        from 'vue'

import { data as tools }   from '../../../../data/tools.data'
import { lookup }          from '../../../../lib/shared/lookup'

interface Credit {
  note   : string
  role   : string
  slug  ?: 'rust' | 'ruff' | 'uv' | 'precommit' | 'vitepress'
  name  ?: string
}

const credits: readonly Credit[] = [
  { slug: 'rust',      role: 'Implementation language',  note: 'Rewriter, parser glue, edit pipeline.' },
  { slug: 'ruff',      role: 'Token upstream',            note: 'Lossless lexer and parser, leased whole.' },
  { slug: 'uv',        role: 'Install path',              note: 'Canonical user entry, wheel-first.' },
  { name: 'mise',      role: 'Tool versions',             note: 'Pinned toolchain, task runner.' },
  { slug: 'precommit', role: 'Commit boundary',           note: 'Hook framework for repo gating.' },
  { slug: 'vitepress', role: 'Docs site',                 note: 'This site, this page, this colophon.' }
]

const entries = computed(() =>
  credits.map((c) => ({
    ...c,
    tool: c.slug ? lookup(tools.entries, c.slug, 'StackColophon credit') : null
  }))
)
</script>

<template>
  <section class="stack-colophon">
    <header class="stack-colophon-head">
      <p class="stack-colophon-kicker">— The Stack —</p>
      <h2 class="stack-colophon-heading">Standing on <em>giants</em>.</h2>
    </header>

    <ul class="stack-colophon-list">
      <li
        v-for="entry in entries"
        :key="entry.slug ?? entry.name"
        class="stack-colophon-row"
      >
        <component
          :is="entry.tool ? 'a' : 'span'"
          class="stack-colophon-name"
          :href="entry.tool?.href"
          :target="entry.tool ? '_blank' : null"
          :rel="entry.tool ? 'noopener' : null"
        >
          <svg
            v-if="entry.tool"
            class="stack-colophon-glyph"
            :viewBox="entry.tool.icon.viewBox"
            aria-hidden="true"
            v-html="entry.tool.icon.body"
          />
          <span class="stack-colophon-wordmark">{{ entry.tool?.name ?? entry.name }}</span>
        </component>
        <span class="stack-colophon-role">{{ entry.role }}</span>
        <span class="stack-colophon-leader" aria-hidden="true" />
        <span class="stack-colophon-note">{{ entry.note }}</span>
      </li>
    </ul>
  </section>
</template>
