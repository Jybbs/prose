import { SITE_HOSTNAME } from '../shared/constants.ts'
import { stripSuffix }   from '../shared/strip-suffix.ts'

export function canonicalUrl(relativePath: string): string {
  return `${SITE_HOSTNAME}/${pagePath(relativePath)}`
}

function pagePath(relativePath: string): string {
  const route = stripSuffix(relativePath, '.md')
  if (route === 'index') return ''
  return route.endsWith('/index') ? stripSuffix(route, 'index') : route
}
