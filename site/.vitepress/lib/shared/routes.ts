import type { RuleFamily, SectionSlug } from './registries'

export function familyRoute(family: RuleFamily): string {
  return `${sectionRoute('rules')}${family}/`
}

export function primitiveRoute(slug: string): string {
  return `/primitives/${slug}`
}

export function ruleRoute(family: RuleFamily, slug: string): string {
  return `${familyRoute(family)}${slug}`
}

export function sectionRoute(slug: SectionSlug): string {
  return `/${slug}/`
}
