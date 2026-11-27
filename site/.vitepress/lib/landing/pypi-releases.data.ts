import { defineLoader } from 'vitepress'

import { conditionalFetch } from '../shared/conditional-fetch.ts'
import { PYPI_PACKAGE }     from '../shared/constants.ts'
import { fetchCacheDir }    from '../shared/paths.ts'

interface PyPIPayload {
  files : readonly PyPIReleaseFile[]
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
  filename      : string
  'upload-time' : string
  yanked?       : boolean | string
}

declare const data: readonly PyPIRelease[]
export { data }

const ENDPOINT  = `https://pypi.org/simple/${PYPI_PACKAGE}/`
const FILE_STEM = `${PYPI_PACKAGE.replaceAll(/[-_.]+/g, '_')}-`
const MONTH_FMT = new Intl.DateTimeFormat('en', { month: 'short', timeZone: 'UTC' })

function compareDesc(a: PyPIRelease, b: PyPIRelease): number {
  return b.date.localeCompare(a.date)
      || b.version.localeCompare(a.version, undefined, { numeric: true })
}

// A distribution filename is `{name}-{version}-{tags}.whl` or
// `{name}-{version}.tar.gz`, with `-` never inside the normalized version.
function versionOf(filename: string): string {
  const rest = filename.slice(FILE_STEM.length)
  return filename.endsWith('.whl') ? rest.split('-')[0] : rest.replace(/\.(?:tar\.gz|zip)$/, '')
}

// The Index API's project detail (https://docs.pypi.org/api/index-api/) lists
// one entry per file, with `yanked` absent or `false` for a live file and
// `true` or a reason string for a yanked one. A version whose every file is
// yanked is left out, and the rest are dated by their first live file.
function toReleases(payload: unknown): readonly PyPIRelease[] {
  const live = new Map<string, PyPIReleaseFile>()
  for (const file of (payload as PyPIPayload).files) {
    const version = versionOf(file.filename)
    if ((file.yanked ?? false) === false && !live.has(version)) live.set(version, file)
  }
  return [...live].map(([version, file]) => {
    const date = file['upload-time'].slice(0, 10)
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
      headers  : { Accept: 'application/vnd.pypi.simple.v1+json' },
      key      : 'pypi-releases',
      parse    : toReleases,
      url      : ENDPOINT
    })
  }
})
