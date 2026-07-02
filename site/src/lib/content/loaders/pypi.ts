import type { Loader } from 'astro/loaders'

import { conditionalLoad } from './conditional'

const PACKAGE   = 'prose-formatter'
const ENDPOINT  = `https://pypi.org/pypi/${PACKAGE}/json`
const MONTH_FMT = new Intl.DateTimeFormat('en', { month: 'short', timeZone: 'UTC' })

interface ReleaseFile {
  upload_time : string
  yanked     ?: boolean
}

interface Payload {
  releases : Record<string, readonly ReleaseFile[]>
}

const projectUrl = (version: string): string => `https://pypi.org/project/${PACKAGE}/${version}/`

// Folds a version and its upload date into the presentational fields each
// release row renders, the month abbreviation and the two-digit year stored
// rather than derived at render time.
function render(version: string, date: string): Record<string, string> {
  const parsed = new Date(date)
  const month  = Number.isNaN(parsed.getTime()) ? '—' : MONTH_FMT.format(parsed).toUpperCase()
  return {
    date,
    month,
    url       : projectUrl(version),
    version,
    year      : date.slice(0, 4),
    yearShort : date.slice(2, 4)
  }
}

const FALLBACK = ([['0.2.0', '2026-04-09'], ['0.1.0', '2026-01-14']] as const)
  .map(([version, date]) => ({ data: render(version, date), id: version }))

function toEntries(payload: unknown): { data: Record<string, string>, id: string }[] {
  return Object.entries((payload as Payload).releases)
    .filter(([, files]) => files.length > 0)
    .map(([version, files]) => {
      const live = files.find(file => !file.yanked) ?? files[0]
      return { data: render(version, live.upload_time.slice(0, 10)), id: version }
    })
    .sort((a, b) => b.data.date.localeCompare(a.data.date))
}

// Loads every PyPI release newest-first, the cold offline build falling back to
// the shipped releases.
export function pypiReleasesLoader(): Loader {
  return {
    name: 'prose-pypi-releases',
    load: ctx => conditionalLoad(ctx, {
      etagKey   : 'pypi-releases:etag',
      fallback  : FALLBACK,
      headers   : { Accept: 'application/json' },
      label     : 'pypi-releases',
      toEntries,
      url       : ENDPOINT
    })
  }
}
