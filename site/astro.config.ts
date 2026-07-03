import { defineConfig, fontProviders } from 'astro/config'
import sitemap                         from '@astrojs/sitemap'
import starlight                       from '@astrojs/starlight'
import postcssGlobalData               from '@csstools/postcss-global-data'
import icon                            from 'astro-icon'
import postcssCustomMedia              from 'postcss-custom-media'
import starlightLinksValidator         from 'starlight-links-validator'

import { buildContentTimestamps, lastmodForUrl } from './src/lib/config/page-timestamps'
import { watchCrateSources }                     from './src/lib/integrations/watch-crate'
import { proseProcessor, shikiConfig }           from './src/lib/markdown/config'
import { sidebar }                               from './src/lib/nav/sidebar'
import { REPO_URL }                              from './src/lib/shared/constants'
import { FONT_FAMILIES }                         from './src/lib/tokens/fonts'
import { resolveColor, tokensToCss }             from './src/lib/tokens/resolve'

const timestamps = buildContentTimestamps(new URL('./', import.meta.url))
const npmLocal   = fontProviders.npm({ remote: false })

export default defineConfig({
  site     : 'https://prose.fyi',
  fonts    : FONT_FAMILIES.map(face => ({ ...face, provider: npmLocal })),
  markdown : { processor: proseProcessor, shikiConfig },

  integrations: [
    starlight({
      customCss       : [
        './src/styles/tokens.css',
        './src/styles/accents.css',
        './src/styles/marks.css',
        './src/styles/markdown.css',
        './src/styles/pq-rows.css',
        './src/styles/primitives.css',
        './src/styles/theme.css'
      ],
      editLink        : { baseUrl: `${REPO_URL}/edit/main/site/` },
      lastUpdated     : true,
      logo            : { alt: 'prose', src: './public/logo.svg' },
      plugins         : [starlightLinksValidator()],
      routeMiddleware : ['./src/lib/head/middleware.ts', './src/lib/nav/middleware.ts'],
      sidebar         : sidebar,
      social          : [{ href: REPO_URL, icon: 'github', label: 'GitHub' }],
      title           : 'Prose',
      titleDelimiter  : '·',

      components: {
        Footer      : './src/components/chrome/Footer.astro',
        Head        : './src/components/chrome/Head.astro',
        SocialIcons : './src/components/chrome/SocialIcons.astro'
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
          postcssGlobalData({ files: ['./src/styles/breakpoints.css'] }),
          postcssCustomMedia()
        ]
      }
    }
  }
})
