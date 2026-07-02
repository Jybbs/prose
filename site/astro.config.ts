import { defineConfig, fontProviders } from 'astro/config'
import sitemap                         from '@astrojs/sitemap'
import starlight                       from '@astrojs/starlight'
import postcssGlobalData               from '@csstools/postcss-global-data'
import icon                            from 'astro-icon'
import postcssCustomMedia              from 'postcss-custom-media'
import starlightLinksValidator         from 'starlight-links-validator'

import { buildContentTimestamps, lastmodForUrl }       from './src/lib/config/page-timestamps'
import { watchCrateSources }                           from './src/lib/integrations/watch-crate'
import { lintFlagPlugin, proseProcessor, shikiConfig } from './src/lib/markdown/config'
import { sidebar }                                     from './src/lib/nav/sidebar'
import { REPO_URL }                                    from './src/lib/shared/constants'
import { FONT_FAMILIES }                               from './src/lib/tokens/fonts'
import { resolveColor, tokensToCss }                   from './src/lib/tokens/resolve'

const timestamps = buildContentTimestamps(new URL('./', import.meta.url))
const npmLocal   = fontProviders.npm({ remote: false })

export default defineConfig({
  site     : 'https://prose.fyi',
  fonts    : FONT_FAMILIES.map(face => ({ ...face, provider: npmLocal })),
  markdown : { processor: proseProcessor, shikiConfig },

  integrations: [
    starlight({
      customCss       : ['./src/styles/theme.css'],
      editLink        : { baseUrl: `${REPO_URL}/edit/main/site/` },
      expressiveCode  : { plugins: [lintFlagPlugin] },
      lastUpdated     : true,
      logo            : { alt: 'prose', src: './public/logo.svg' },
      plugins         : [starlightLinksValidator()],
      routeMiddleware : ['./src/lib/head/middleware.ts', './src/lib/nav/middleware.ts'],
      sidebar         : sidebar,
      social          : [{ href: REPO_URL, icon: 'github', label: 'GitHub' }],
      title           : 'Prose',
      titleDelimiter  : '·',

      components: {
        Head        : './src/components/Head.astro',
        SocialIcons : './src/components/SocialIcons.astro'
      },

      head: [
        {
          attrs : { content: resolveColor('palette-ube'), name: 'theme-color' },
          tag   : 'meta'
        },
        {
          content : tokensToCss(),
          tag     : 'style'
        }
      ]
    }),
    icon(),
    sitemap({
      serialize(item) {
        const lastmod = lastmodForUrl(item.url, timestamps)
        return lastmod ? { ...item, lastmod } : item
      }
    }),
    watchCrateSources()
  ],

  vite: {
    css: {
      postcss: {
        plugins: [
          // `postcssGlobalData` must precede `postcssCustomMedia` so the
          // breakpoint definitions are in scope when the queries resolve.
          postcssGlobalData({ files: ['./src/styles/breakpoints.css'] }),
          postcssCustomMedia()
        ]
      }
    }
  }
})
