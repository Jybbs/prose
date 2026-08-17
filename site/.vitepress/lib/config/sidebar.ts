import path from 'node:path'

import type { DefaultTheme } from 'vitepress'

import { markdownH1 }               from '../markdown/h1'
import type { DiscoveredPrimitive } from '../primitives/discovery'
import type { DiscoveredRule }      from '../rules/discovery'
import { matterPages }              from '../shared/content-page'
import * as registries              from '../shared/registries'
import { requireString }            from '../shared/require-string'
import * as routes                  from '../shared/routes'

type SidebarPrimitive = Pick<DiscoveredPrimitive, 'name' | 'slug' | 'stability'>

const primLink = (slug: string, text: string): DefaultTheme.SidebarItem =>
  ({ link: routes.primitiveRoute(slug), text })

const ruleLink = (rule: DiscoveredRule): DefaultTheme.SidebarItem =>
  ({ link: rule.href, text: rule.slug })

export function buildSidebar(
  primitives : readonly SidebarPrimitive[],
  rules      : readonly DiscoveredRule[],
  srcDir     : string
): DefaultTheme.Sidebar {
  return Object.fromEntries(registries.SECTIONS.map(({ label, slug }) =>
    [routes.sectionRoute(slug), sectionGroups(label, primitives, rules, slug, srcDir)]))
}

function primitiveGroups(
  label      : string,
  primitives : readonly SidebarPrimitive[]
): DefaultTheme.SidebarItem[] {
  const publicPrimitives   = primitives.filter(p => p.stability === 'public')
  const internalPrimitives = primitives.filter(p => p.stability === 'internal')
  return [
    { items: [{ link: '/primitives/', text: 'Overview' }], text: label },
    {
      items : publicPrimitives.map(p => primLink(p.slug, p.name)),
      text  : 'Public Surface'
    },
    {
      items : internalPrimitives.map(p => primLink(p.slug, p.name)),
      text  : 'Crate Internal'
    }
  ]
}

function ruleGroups(label: string, rules: readonly DiscoveredRule[]): DefaultTheme.SidebarItem[] {
  const familySections: DefaultTheme.SidebarItem[] = registries.FAMILY_ORDER.map(family => ({
    items : rules
      .filter(r => r.family === family)
      .map(ruleLink),
    link  : routes.familyRoute(family),
    text  : registries.FAMILY_META[family].label
  }))
  return [
    {
      items: [
        { link: '/rules/',             text: 'Overview'    },
        { link: '/rules/composition/', text: 'Composition' }
      ],
      text : label
    },
    ...familySections
  ]
}

function sectionGroups(
  label      : string,
  primitives : readonly SidebarPrimitive[],
  rules      : readonly DiscoveredRule[],
  slug       : registries.SectionSlug,
  srcDir     : string
): DefaultTheme.SidebarItem[] {
  switch (slug) {
    case 'primitives' : return primitiveGroups(label, primitives)
    case 'rules'      : return ruleGroups(label, rules)
    case 'sandbox'    : return []
    default           : return [{
      items : [
        { link: routes.sectionRoute(slug), text: 'Overview' },
        ...sectionPages(path.join(srcDir, slug), slug)
      ],
      text  : label
    }]
  }
}

function sectionPages(directory: string, slug: registries.SectionSlug): DefaultTheme.SidebarItem[] {
  return matterPages(directory).map(page => {
    const title = requireString(
      markdownH1(page.content),
      `${slug}/${page.file} has no H1 for its sidebar entry`
    )
    return { link: `${routes.sectionRoute(slug)}${page.slug}`, text: title }
  })
}
