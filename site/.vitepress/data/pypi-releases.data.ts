import { defineLoader } from 'vitepress'

import { withFallback } from '../lib/shared/with-fallback'

export interface PyPIRelease {
  date      : string
  month     : string
  url       : string
  version   : string
  year      : string
  yearShort : string
}

declare const data: readonly PyPIRelease[]
export { data }

interface PyPIReleaseFile {
  upload_time : string
  yanked     ?: boolean
}

interface PyPIPayload {
  releases : Record<string, readonly PyPIReleaseFile[]>
}

const PACKAGE   = 'prose-formatter'
const ENDPOINT  = `https://pypi.org/pypi/${PACKAGE}/json`
const MONTH_FMT = new Intl.DateTimeFormat('en', { month: 'short', timeZone: 'UTC' })

function projectUrl(version: string): string {
  return `https://pypi.org/project/${PACKAGE}/${version}/`
}

function render(version: string, date: string): PyPIRelease {
  const d     = new Date(date)
  const month = Number.isNaN(d.getTime()) ? '—' : MONTH_FMT.format(d).toUpperCase()
  return {
    date,
    month,
    url       : projectUrl(version),
    version,
    year      : date.slice(0, 4),
    yearShort : date.slice(2, 4)
  }
}

const FALLBACK: readonly PyPIRelease[] = (
  [['0.2.0', '2026-04-09'], ['0.1.0', '2026-01-14']] as const
).map(([version, date]) => render(version, date))

function compareDesc(a: PyPIRelease, b: PyPIRelease): number {
  return b.date.localeCompare(a.date)
      || b.version.localeCompare(a.version, undefined, { numeric: true })
}

export default defineLoader({
  watch: [],
  async load(): Promise<readonly PyPIRelease[]> {
    return withFallback('pypi-releases:fetch', async () => {
      const response = await fetch(ENDPOINT, { headers: { Accept: 'application/json' } })
      if (!response.ok) throw new Error(`PyPI returned ${response.status}`)
      const payload = await response.json() as PyPIPayload
      const entries = Object.entries(payload.releases)
        .filter(([, files]) => files && files.length > 0)
        .map(([version, files]) => {
          const live = files.find(f => !f.yanked) ?? files[0]
          return render(version, live.upload_time.slice(0, 10))
        })
        .toSorted(compareDesc)
      return entries.length > 0 ? entries : FALLBACK
    }, FALLBACK)
  }
})
