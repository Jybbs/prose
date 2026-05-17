import fs   from 'node:fs'
import path from 'node:path'

import { defineConfig, type DefaultTheme } from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin } from 'vitepress-plugin-tabs'

import { REPO_URL, SITE_HOSTNAME } from './lib/constants'
import { repoRoot, rulesDir }      from './lib/paths'
import { PRIMITIVES }              from './lib/primitives'
import { ruleLinkPlugin }          from './lib/rule-link'
import { SHIKI_THEMES }            from './lib/shiki'
import { readCargoVersion }        from './lib/version'

const repoDir = repoRoot(import.meta.url)
const version = readCargoVersion(repoDir)

const primLink = (text: string, slug: string): DefaultTheme.SidebarItem =>
  ({ text, link: `/primitives/${slug}` })

const ruleLink = (slug: string): DefaultTheme.SidebarItem =>
  ({ text: slug, link: `/rules/${slug}` })

function sidebarRules(dir: string): { autoFix: string[]; lint: string[] } {
  const autoFix: string[] = []
  const lint   : string[] = []
  for (const file of fs.readdirSync(dir).sort()) {
    if (!file.endsWith('.md') || file === 'index.md') continue
    const body  = fs.readFileSync(path.join(dir, file), 'utf8')
    const front = body.match(/^---\n([\s\S]*?)\n---\n/)?.[1] ?? ''
    const slug  = file.slice(0, -'.md'.length)
    if (/^\s*category:\s*lint\s*$/m.test(front)) lint.push(slug)
    else                                          autoFix.push(slug)
  }
  return { autoFix, lint }
}

const { autoFix, lint } = sidebarRules(rulesDir(import.meta.url))
const validSlugs        = new Set([...autoFix, ...lint])

export default defineConfig({
  cleanUrls     : true,
  description   : 'A Python typesetter for the reader.',
  head          : [
    ['link', { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.googleapis.com' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: '' }],
    ['link', {
      rel : 'stylesheet',
      href: 'https://fonts.googleapis.com/css2?family=Fraunces:ital,opsz,wght@0,9..144,400;0,9..144,500;0,9..144,600;0,9..144,700;1,9..144,400;1,9..144,500;1,9..144,600&family=JetBrains+Mono:ital,wght@0,400;0,500;0,700;1,400;1,500&family=Lora:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500&display=swap'
    }]
  ],
  lastUpdated   : true,
  markdown      : {
    config     : md => {
      md.use(groupIconMdPlugin)
      md.use(tabsMarkdownPlugin)
      md.use(ruleLinkPlugin(validSlugs))
    },
    lineNumbers: false,
    theme      : SHIKI_THEMES
  },
  sitemap       : {
    hostname: SITE_HOSTNAME
  },
  title         : 'Prose',
  titleTemplate : ':title · Prose',
  transformPageData(pageData) {
    pageData.frontmatter ||= {}
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push([
      'link',
      { rel: 'canonical', href: `${SITE_HOSTNAME}/${pageData.relativePath.replace(/(^|\/)index\.md$/, '$1').replace(/\.md$/, '')}` }
    ])
  },
  vite          : {
    plugins: [groupIconVitePlugin() as never]
  },
  themeConfig: {
    editLink   : {
      pattern: `${REPO_URL}/edit/main/site/:path`,
      text   : 'Suggest an edit to this page'
    },
    footer     : {
      copyright: '© Jybbs',
      message  : 'Released under the MIT License.'
    },
    logo       : { src: '/logo.svg', alt: 'prose' },
    nav        : [
      { text: 'Guide',       link: '/guide/installation', activeMatch: '/guide/'      },
      { text: 'Rules',       link: '/rules/',             activeMatch: '/rules/'      },
      { text: 'Primitives',  link: '/primitives/source',  activeMatch: '/primitives/' },
      { text: `v${version}`, link: `${REPO_URL}/releases` }
    ],
    outline    : { level: [2, 3] },
    search     : { provider: 'local' },
    sidebar    : {
      '/guide/': [
        {
          text : 'Guide',
          items: [
            { text: 'Installation',       link: '/guide/installation'       },
            { text: 'Configuration',      link: '/guide/configuration'      },
            { text: 'Suppression',        link: '/guide/suppression'        },
            { text: 'Editor Integration', link: '/guide/editor-integration' },
            { text: 'CI Integration',     link: '/guide/ci-integration'     }
          ]
        }
      ],
      '/rules/': [
        { text: 'Rules',    items: [{ text: 'Overview', link: '/rules/' }] },
        { text: 'Auto-Fix', items: autoFix.map(ruleLink) },
        { text: 'Lint',     items: lint.map(ruleLink) }
      ],
      '/primitives/': [
        {
          text : 'Primitives',
          items: Object.entries(PRIMITIVES).map(([slug, label]) => primLink(label, slug))
        }
      ]
    },
    siteTitle  : 'Prose',
    socialLinks: [
      { icon: 'github', link: REPO_URL }
    ]
  }
})
