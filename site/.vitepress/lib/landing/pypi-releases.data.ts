import { defineLoader } from 'vitepress'

import { conditionalFetch } from '../shared/conditional-fetch'
import { PYPI_PACKAGE }     from '../shared/constants'
import { fetchCacheDir }    from '../shared/paths'

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

const ENDPOINT  = `https://pypi.org/pypi/${PYPI_PACKAGE}/json`
const MONTH_FMT = new Intl.DateTimeFormat('en', { month: 'short', timeZone: 'UTC' })

function projectUrl(version: string): string {
  return `https://pypi.org/project/${PYPI_PACKAGE}/${version}/`
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
  [['0.8.1', '2026-07-16'], ['0.8.0', '2026-07-13']] as const
).map(([version, date]) => render(version, date))

function compareDesc(a: PyPIRelease, b: PyPIRelease): number {
  return b.date.localeCompare(a.date)
      || b.version.localeCompare(a.version, undefined, { numeric: true })
}

function toReleases(payload: unknown): readonly PyPIRelease[] {
  const entries = Object.entries((payload as PyPIPayload).releases)
    .filter(([, files]) => files && files.length > 0)
    .map(([version, files]) => {
      const live = files.find(f => !f.yanked) ?? files[0]
      return render(version, live.upload_time.slice(0, 10))
    })
    .toSorted(compareDesc)
  return entries.length > 0 ? entries : FALLBACK
}

export default defineLoader({
  watch: [],
  load(): Promise<readonly PyPIRelease[]> {
    return conditionalFetch({
      dir      : fetchCacheDir(import.meta.url),
      fallback : FALLBACK,
      headers  : { Accept: 'application/json' },
      key      : 'pypi-releases',
      parse    : toReleases,
      url      : ENDPOINT
    })
  }
})
