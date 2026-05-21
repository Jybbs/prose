<script setup lang="ts">
import LandingSection      from './LandingSection.vue'

import { data as tools }   from '../../../data/tools.data'
import { lookup }          from '../../../lib/shared/lookup'

interface Credit {
  role : string
  slug : 'rust' | 'ruff' | 'uv' | 'mise' | 'github'
}

const credits: readonly Credit[] = [
  { role: 'CI',             slug: 'github' },
  { role: 'Implementation', slug: 'rust'   },
  { role: 'Install path',   slug: 'uv'     },
  { role: 'Tool versions',  slug: 'mise'   },
  { role: 'Upstream pass',  slug: 'ruff'   }
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
        <span class="built-on-medallion">
          <svg
            class="built-on-logo"
            :viewBox="entry.tool.icon.viewBox"
            aria-hidden="true"
            v-html="entry.tool.icon.body"
          />
        </span>
        <span class="built-on-name">{{ entry.tool.name }}</span>
        <span class="built-on-role kicker">{{ entry.role }}</span>
      </a>
    </div>
  </LandingSection>
</template>
