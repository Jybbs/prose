import { defineLoader } from 'vitepress'

import { conditionalFetch } from '../shared/conditional-fetch.ts'
import { PYPI_PACKAGE }     from '../shared/constants.ts'
import { fetchCacheDir }    from '../shared/paths.ts'

interface PyPIPayload {
  releases : Record<string, readonly PyPIReleaseFile[]>
}

export interface PyPIRelease {
  date      : string
  month     : string
  url       : string
  version   : string
  year      : string
  yearShort : string
}

interface PyPIReleaseFile {
  upload_time : string
  yanked?     : boolean
}

declare const data: readonly PyPIRelease[]
export { data }

const ENDPOINT  = `https://pypi.org/pypi/${PYPI_PACKAGE}/json`
const MONTH_FMT = new Intl.DateTimeFormat('en', { month: 'short', timeZone: 'UTC' })

function compareDesc(a: PyPIRelease, b: PyPIRelease): number {
  return b.date.localeCompare(a.date)
      || b.version.localeCompare(a.version, undefined, { numeric: true })
}

function toReleases(payload: unknown): readonly PyPIRelease[] {
  return Object.entries((payload as PyPIPayload).releases)
    .filter(([, files]) => files.length > 0)
    .map(([version, files]) => {
      const date = (files.find(f => !f.yanked) ?? files[0]).upload_time.slice(0, 10)
      return {
        date      : date,
        month     : MONTH_FMT.format(new Date(date)).toUpperCase(),
        url       : `https://pypi.org/project/${PYPI_PACKAGE}/${version}/`,
        version   : version,
        year      : date.slice(0, 4),
        yearShort : date.slice(2, 4)
      }
    })
    .toSorted(compareDesc)
}

export default defineLoader({
  watch: [],
  load(): Promise<readonly PyPIRelease[]> {
    return conditionalFetch({
      dir      : fetchCacheDir(import.meta.url),
      fallback : [],
      headers  : { Accept: 'application/json' },
      key      : 'pypi-releases',
      parse    : toReleases,
      url      : ENDPOINT
    })
  }
})
