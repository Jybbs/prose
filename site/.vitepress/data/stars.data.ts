import { defineLoader } from 'vitepress'

export interface StarsData {
  stars: string
}

declare const data: StarsData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<StarsData> {
    const fallback: StarsData = { stars: '0' }
    if (process.env.PROSE_OFFLINE_DOCS === '1') return fallback
    try {
      const response = await fetch('https://api.github.com/repos/Jybbs/prose', {
        headers: { 'User-Agent': 'prose-docs-build' }
      })
      if (!response.ok) return fallback
      const body = await response.json() as { stargazers_count?: number }
      const count = body.stargazers_count ?? 0
      return { stars: formatStars(count) }
    } catch {
      return fallback
    }
  }
})

function formatStars(count: number): string {
  if (count < 1000) return String(count)
  return `${(count / 1000).toFixed(1).replace(/\.0$/, '')}k`
}
