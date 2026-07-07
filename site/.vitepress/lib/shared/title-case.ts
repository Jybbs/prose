import { titleCase } from 'title-case'

export function toTitleCase(slug: string, separator = '_'): string {
  return titleCase(slug.replaceAll(separator, ' '))
}
