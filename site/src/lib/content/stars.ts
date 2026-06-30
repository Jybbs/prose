import type { Loader } from 'astro/loaders'

import { conditionalLoad } from './conditional'

const ENDPOINT = 'https://api.github.com/repos/Jybbs/prose'

const stargazers = (payload: unknown): number =>
  (payload as { stargazers_count?: number }).stargazers_count ?? 0

// Collapses a thousands count to a `1.2k` form.
const formatStars = (count: number): string =>
  count < 1000 ? String(count) : `${(count / 1000).toFixed(1).replace(/\.0$/, '')}k`

// Loads the GitHub stargazer count as a single entry, the cold offline build
// falling back to `0`.
export function starsLoader(): Loader {
  return {
    name: 'prose-stars',
    load: ctx => conditionalLoad(ctx, {
      etagKey   : 'stars:etag',
      fallback  : [{ data: { stars: '0' }, id: 'stars' }],
      headers   : { 'User-Agent': 'prose-docs-build' },
      label     : 'stars',
      toEntries : payload => [{ data: { stars: formatStars(stargazers(payload)) }, id: 'stars' }],
      url       : ENDPOINT
    })
  }
}
