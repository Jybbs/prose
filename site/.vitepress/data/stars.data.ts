import { defineLoader } from 'vitepress'

import { withFallback } from '../lib/shared/with-fallback'

interface StarsData {
  stars: string
}

declare const data: StarsData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<StarsData> {
    if (process.env.PROSE_OFFLINE_DOCS === '1') return { stars: '0' }
    const stars = await withFallback('stars:fetch', async () => {
      const response = await fetch('https://api.github.com/repos/Jybbs/prose', {
        headers: { 'User-Agent': 'prose-docs-build' }
      })
      if (!response.ok) throw new Error(`GitHub API returned ${response.status}`)
      const body  = await response.json() as { stargazers_count?: number }
      const count = body.stargazers_count ?? 0
      return count < 1000
        ? String(count)
        : `${(count / 1000).toFixed(1).replace(/\.0$/, '')}k`
    }, '0')
    return { stars }
  }
})
