import { defineLoader } from 'vitepress'

import { conditionalFetch } from './conditional-fetch'
import { REPO_SLUG }        from './constants'
import { fetchCacheDir }    from './paths'

interface StarsData {
  stars: string
}

const STAR_FMT = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 })

declare const data: StarsData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<StarsData> {
    const count = await conditionalFetch({
      dir      : fetchCacheDir(import.meta.url),
      fallback : 0,
      headers  : { 'User-Agent': 'prose-docs-build' },
      key      : 'stars',
      parse    : payload => (payload as { stargazers_count?: number }).stargazers_count ?? 0,
      url      : `https://api.github.com/repos/${REPO_SLUG}`
    })
    return { stars: STAR_FMT.format(count).toLowerCase() }
  }
})
