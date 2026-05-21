<script setup lang="ts">
import LandingSection      from './LandingSection.vue'

import { data as tools }   from '../../../data/tools.data'
import { lookup }          from '../../../lib/shared/lookup'

interface Credit {
  role : string
  slug : 'rust' | 'ruff' | 'uv' | 'maturin' | 'precommit' | 'vitepress'
}

const credits: readonly Credit[] = [
  { role: 'Implementation language',         slug: 'rust'       },
  { role: 'Token-level upstream pass',       slug: 'ruff'       },
  { role: 'Canonical install path',          slug: 'uv'         },
  { role: 'Rust-to-Python wheel build',      slug: 'maturin'    },
  { role: 'Commit-boundary hook framework',  slug: 'precommit'  },
  { role: 'Docs site framework',             slug: 'vitepress'  }
]

const entries = credits.map(({ role, slug }) => ({
  role,
  slug,
  tool: lookup(tools.entries, slug, 'BuiltOn credit')
}))
</script>

<template>
  <LandingSection
    centered
    variant="built-on"
    kicker="The Lineage"
    heading="Standing on <em>giants</em>."
  >
    <div class="built-on-grid">
      <a
        v-for="entry in entries"
        :key="entry.slug"
        class="built-on-card"
        :href="entry.tool.href"
        target="_blank"
        rel="noopener"
      >
        <svg
          class="built-on-logo"
          :viewBox="entry.tool.icon.viewBox"
          aria-hidden="true"
          v-html="entry.tool.icon.body"
        />
        <span class="built-on-name">{{ entry.tool.name }}</span>
        <span class="built-on-role">{{ entry.role }}</span>
      </a>
    </div>
    <p class="built-on-aside">
      Plus <code>mise</code> for tool versions and tasks, <code>cargo</code> for the crate workspace, and GitHub Actions for CI.
    </p>
  </LandingSection>
</template>
