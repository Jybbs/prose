import { pagePath } from './canonical-url'

export function attachLastmod<T extends { lastmod?: string | number | Date, url: string }>(
  items      : readonly T[],
  timestamps : ReadonlyMap<string, number>
): T[] {
  const byUrl = new Map([...timestamps].map(([page, ms]) => [pagePath(page), ms]))
  return items.map(item => {
    const url     = item.url.endsWith('.html') ? item.url.slice(0, -5) : item.url
    const lastmod = byUrl.get(url)
    return lastmod === undefined ? item : { ...item, lastmod }
  })
}
