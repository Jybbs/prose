import { defineConfig }        from 'astro/config'
import sitemap                 from '@astrojs/sitemap'
import starlight               from '@astrojs/starlight'
import starlightLinksValidator from 'starlight-links-validator'

import { buildContentTimestamps, lastmodForUrl }       from './src/lib/config/page-timestamps'
import { watchCrateSources }                           from './src/lib/integrations/watch-crate'
import { lintFlagPlugin, proseProcessor, shikiConfig } from './src/lib/markdown/config'
import { REPO_URL }                                    from './src/lib/shared/constants'
import { resolveColor }                                from './src/lib/tokens/resolve'

const timestamps = buildContentTimestamps(new URL('./', import.meta.url))

export default defineConfig({
  site         : 'https://prose.fyi',
  markdown     : { processor: proseProcessor, shikiConfig },
  integrations : [
    starlight({
      title           : 'Prose',
      components      : { SocialIcons: './src/components/SocialIcons.astro' },
      editLink        : { baseUrl: `${REPO_URL}/edit/main/site/` },
      expressiveCode  : { plugins: [lintFlagPlugin] },
      lastUpdated     : true,
      plugins         : [starlightLinksValidator()],
      routeMiddleware : './src/lib/head/middleware.ts',
      titleDelimiter  : '·',
      head            : [{
        attrs : { content: resolveColor('palette-ube'), name: 'theme-color' },
        tag   : 'meta'
      }]
    }),
    sitemap({
      serialize(item) {
        const lastmod = lastmodForUrl(item.url, timestamps)
        return lastmod ? { ...item, lastmod } : item
      }
    }),
    watchCrateSources()
  ]
})
