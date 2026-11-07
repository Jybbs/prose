import { defineComponent } from 'vue'

import { ToolIcon } from '../../../kit'

import { LandingSection } from './LandingSection'

import { data as tools } from '../../../lib/shared/tools.data'
import { externalAttrs } from '../../../lib/shared/links'
import type { ToolSlug } from '../../../lib/shared/tools'

interface Credit {
  role : string
  slug : ToolSlug
}

export const BuiltOn = defineComponent({
  name: 'BuiltOn',
  setup() {
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
      tool: tools.entries[slug]
    }))

    return () => (
      <LandingSection centered heading={"Standing on <em>giants</em>."} kicker="The Lineage">
        <div class="built-on-grid mt-24 grid grid-cols-5 overflow-hidden panel bg-bg-soft max-medium:grid-cols-3 max-tiny:grid-cols-1fr">
          {entries.map(entry => (
            <a
              href={entry.tool.href}
              key={entry.slug}
              {...externalAttrs(entry.tool.href)}
              class="built-on-card group/card grid grid-rows-64-28-18 content-start justify-items-center gap-y-8 px-14 py-22 text-center text-text-1 transition-colors duration-slow hover:bg-stack-accent/faint max-tiny:not-first:border-t-1 max-tiny:not-first:border-l-0 max-tiny:not-first:border-t-divider"
            >
              <span class="medallion size-56 border-stack-accent-strong-divider bg-stack-accent-faint-bg text-stack-accent-68-text-1 transition-colors duration-slow group-hover/card:border-stack-accent-60-divider group-hover/card:bg-stack-accent-wash-bg group-hover/card:text-stack-accent-82-text-1">
                <ToolIcon class="size-28" slug={entry.slug} />
              </span>
              <span class="font-display text-1-15rem leading-none font-500 italic">{entry.tool.name}</span>
              <span class="kicker leading-none text-ube-soft-45">{entry.role}</span>
            </a>
          ))}
        </div>
      </LandingSection>
    )
  }
})
