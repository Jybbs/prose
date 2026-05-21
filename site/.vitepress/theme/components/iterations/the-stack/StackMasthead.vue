<script setup lang="ts">
import { computed }        from 'vue'

import { data as tools }   from '../../../../data/tools.data'
import { lookup }          from '../../../../lib/shared/lookup'

interface Primary {
  blurb : string
  role  : string
  slug  : 'rust' | 'ruff' | 'uv' | 'mise'
}

interface Secondary {
  role  : string
  slug ?: 'precommit' | 'vitepress'
  name ?: string
}

const primaries: readonly Primary[] = [
  { slug: 'rust', role: 'Implementation', blurb: 'A rewriter, end to end.' },
  { slug: 'ruff', role: 'Upstream pass',  blurb: 'Lossless lex, parse, trivia.' },
  { slug: 'uv',   role: 'Install path',   blurb: 'Wheel-first user entry.' },
  { slug: 'mise', role: 'Tool versions',  blurb: 'Pinned toolchain per repo.' }
]

const secondaries: readonly Secondary[] = [
  { slug: 'precommit', role: 'commit boundary' },
  { slug: 'vitepress', role: 'docs site'       }
]

const primaryEntries = computed(() =>
  primaries.map((p) => ({ ...p, tool: lookup(tools.entries, p.slug, 'StackMasthead primary') }))
)

const secondaryEntries = computed(() =>
  secondaries.map((s) => ({
    ...s,
    tool: s.slug ? lookup(tools.entries, s.slug, 'StackMasthead secondary') : null
  }))
)
</script>

<template>
  <section class="stack-masthead">
    <header class="stack-masthead-head">
      <p class="stack-masthead-kicker">— The Stack —</p>
      <h2 class="stack-masthead-heading">Standing on <em>giants</em>.</h2>
    </header>

    <div class="stack-masthead-row">
      <a
        v-for="entry in primaryEntries"
        :key="entry.slug"
        class="stack-masthead-card"
        :href="entry.tool.href"
        target="_blank"
        rel="noopener"
      >
        <svg
          class="stack-masthead-glyph"
          :viewBox="entry.tool.icon.viewBox"
          aria-hidden="true"
          v-html="entry.tool.icon.body"
        />
        <span class="stack-masthead-name">{{ entry.tool.name }}</span>
        <span class="stack-masthead-role">{{ entry.role }}</span>
        <span class="stack-masthead-blurb">{{ entry.blurb }}</span>
      </a>
    </div>

    <div class="stack-masthead-rule" aria-hidden="true" />

    <p class="stack-masthead-set-in">
      <span class="stack-masthead-prefix">and set in</span>
      <span
        v-for="(entry, idx) in secondaryEntries"
        :key="entry.slug ?? entry.name"
        class="stack-masthead-credit"
      >
        <component
          :is="entry.tool ? 'a' : 'span'"
          class="stack-masthead-credit-mark"
          :href="entry.tool?.href"
          :target="entry.tool ? '_blank' : null"
          :rel="entry.tool ? 'noopener' : null"
        >{{ entry.tool?.name ?? entry.name }}</component>
        <span class="stack-masthead-credit-role">{{ entry.role }}</span>
        <span
          v-if="idx < secondaryEntries.length - 1"
          class="stack-masthead-bullet"
          aria-hidden="true"
        >·</span>
      </span>
    </p>
  </section>
</template>
