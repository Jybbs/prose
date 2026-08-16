import type { RuleFamily, SectionSlug } from './registries'

export function compositionRoute(): string {
  return `${sectionRoute('rules')}composition/`
}

export function familyRoute(family: RuleFamily): string {
  return `/rules/${family}/`
}

export function primitiveRoute(slug: string): string {
  return `/primitives/${slug}`
}

export function ruleRoute(family: RuleFamily, slug: string): string {
  return `/rules/${family}/${slug}`
}

export function sectionRoute(slug: SectionSlug): string {
  return `/${slug}/`
}
