import { defineLoader } from 'vitepress'

import { withFallback } from '../lib/shared/with-fallback'

export interface PyPIRelease {
  date    : string
  url     : string
  version : string
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

const PACKAGE  = 'prose-formatter'
const ENDPOINT = `https://pypi.org/pypi/${PACKAGE}/json`

function projectUrl(version: string): string {
  return `https://pypi.org/project/${PACKAGE}/${version}/`
}

const FALLBACK: readonly PyPIRelease[] = (
  [['0.2.0', '2026-04-09'], ['0.1.0', '2026-01-14']] as const
).map(([version, date]) => ({ date, url: projectUrl(version), version }))

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
          return { date: live.upload_time.slice(0, 10), url: projectUrl(version), version }
        })
        .sort(compareDesc)
      return entries.length > 0 ? entries : FALLBACK
    }, FALLBACK)
  }
})
