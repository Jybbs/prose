<script setup lang="ts">
import LandingSection      from './LandingSection.vue'

import { data as tools } from '../../../lib/shared/tools.data'
import { externalAttrs } from '../../../lib/shared/links'
import { lookup }        from '../../../lib/shared/lookup'

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
    kicker="The Lineage"
    heading="Standing on <em>giants</em>."
  >
    <div class="built-on-grid mt-24 grid grid-cols-5 overflow-hidden panel bg-bg-soft max-medium:grid-cols-3 max-tiny:grid-cols-1fr">
      <a
        v-for="entry in entries"
        :key="entry.slug"
        class="built-on-card group/card grid grid-rows-64-28-18 content-start justify-items-center gap-y-8 px-14 py-22 text-center text-text-1 transition-colors duration-slow hover:bg-stack-accent/faint max-tiny:not-first:border-t-1 max-tiny:not-first:border-l-0 max-tiny:not-first:border-t-divider"
        :href="entry.tool.href"
        v-bind="externalAttrs(entry.tool.href)"
      >
        <span class="medallion size-56 border-stack-accent-strong-divider bg-stack-accent-faint-bg text-stack-accent-68-text-1 transition-colors duration-slow group-hover/card:border-stack-accent-60-divider group-hover/card:bg-stack-accent-wash-bg group-hover/card:text-stack-accent-82-text-1">
          <svg
            class="size-28"
            :viewBox="entry.tool.icon.viewBox"
            aria-hidden="true"
            v-html="entry.tool.icon.body"
          />
        </span>
        <span class="font-display text-1-15rem leading-none font-500 italic">{{ entry.tool.name }}</span>
        <span class="kicker leading-none text-ube-soft-45">{{ entry.role }}</span>
      </a>
    </div>
  </LandingSection>
</template>
