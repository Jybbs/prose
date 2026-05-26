<script setup lang="ts">
import LandingSection      from './LandingSection.vue'

import { data as tools }   from '../../../data/tools.data'
import { externalAttrs }   from '../../../lib/shared/links'
import { lookup }          from '../../../lib/shared/lookup'

interface Credit {
  role : string
  slug : 'github' | 'mise' | 'ruff' | 'rust' | 'uv'
}

const credits: readonly Credit[] = [
  { role: 'CI',             slug: 'github' },
  { role: 'Implementation', slug: 'rust'   },
  { role: 'Install path',   slug: 'uv'     },
  { role: 'Tool versions',  slug: 'mise'   },
  { role: 'Parser & AST',   slug: 'ruff'   }
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
        v-bind="externalAttrs(entry.tool.href)"
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
