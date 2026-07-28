import { defineLoader } from 'vitepress'

import { conditionalFetch }        from '../shared/conditional-fetch'
import { PYPI_PACKAGE }            from '../shared/constants'
import { crateDir, fetchCacheDir } from '../shared/paths'
import { readCargoVersion }        from '../shared/version'

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

const CRATE     = crateDir(import.meta.url)
const ENDPOINT  = `https://pypi.org/pypi/${PYPI_PACKAGE}/json`
const MONTH_FMT = new Intl.DateTimeFormat('en', { month: 'short', timeZone: 'UTC' })

function projectUrl(version: string): string {
  return `https://pypi.org/project/${PYPI_PACKAGE}/${version}/`
}

function render(version: string, date: string): PyPIRelease {
  const d     = new Date(date)
  const dated = !Number.isNaN(d.getTime())
  return {
    date,
    month     : dated ? MONTH_FMT.format(d).toUpperCase() : '—',
    url       : projectUrl(version),
    version,
    year      : dated ? date.slice(0, 4) : '—',
    yearShort : dated ? date.slice(2, 4) : '—'
  }
}

const FALLBACK: readonly PyPIRelease[] = [render(readCargoVersion(CRATE), '')]

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
