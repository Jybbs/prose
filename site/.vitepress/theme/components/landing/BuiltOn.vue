<script setup lang="ts">
import { computed } from 'vue'

import LandingSection      from './LandingSection.vue'
import { data as tools }   from '../../../data/tools.data'
import { lookup }          from '../../../lib/shared/lookup'

interface Source {
  role : string
  slug : string
}

const sources: readonly Source[] = [
  {
    role : 'Implementation language, picked for native binary speed and predictable per-file parallelism on large trees.',
    slug : 'rust'
  },
  {
    role : 'Astral’s Python AST and parser primitives pinned to release 0.15.10, the same surface Ruff itself consumes.',
    slug : 'ruff'
  },
  {
    role : 'CLI parsing and shell-completion generation across bash, zsh, fish, elvish, and powershell.',
    slug : 'clap'
  },
  {
    role : 'Per-file parallelism across the walker output, one rule pipeline per worker thread.',
    slug : 'rayon'
  },
  {
    role : 'Rust to Python wheel build, producing the platform binaries that PyPI ships.',
    slug : 'maturin'
  },
  {
    role : 'Canonical install path, fetching the wheel and exposing the binary on PATH without a venv.',
    slug : 'uv'
  }
]

const credits = computed(() => sources.map(({ role, slug }) => ({
  role,
  slug,
  tool: lookup(tools.entries, slug, 'BuiltOn credit')
})))
</script>

<template>
  <LandingSection variant="built-on" kicker="The Stack" heading="Built on">
    <div class="built-on-grid">
      <a
        v-for="credit in credits"
        :key="credit.slug"
        class="built-on-card"
        :href="credit.tool.href"
        target="_blank"
        rel="noopener"
      >
        <svg class="built-on-icon" :viewBox="credit.tool.icon.viewBox" aria-hidden="true" v-html="credit.tool.icon.body" />
        <h3 class="built-on-name">{{ credit.tool.name }}</h3>
        <p class="built-on-role">{{ credit.role }}</p>
        <span class="built-on-arrow" aria-hidden="true">↗</span>
      </a>
    </div>
  </LandingSection>
</template>

