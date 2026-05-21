import { defineLoader } from 'vitepress'

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

const FALLBACK: readonly PyPIRelease[] = [
  { date : '2026-04-09', url : `https://pypi.org/project/${PACKAGE}/0.2.0/`, version : '0.2.0' },
  { date : '2026-01-14', url : `https://pypi.org/project/${PACKAGE}/0.1.0/`, version : '0.1.0' }
]

function compareDesc(a: PyPIRelease, b: PyPIRelease): number {
  if (a.date !== b.date) return b.date.localeCompare(a.date)
  return compareSemverDesc(b.version, a.version)
}

function compareSemverDesc(a: string, b: string): number {
  const pa = a.split('.').map(n => Number.parseInt(n, 10))
  const pb = b.split('.').map(n => Number.parseInt(n, 10))
  const len = Math.max(pa.length, pb.length)
  for (let i = 0; i < len; i++) {
    const da = pa[i] ?? 0
    const db = pb[i] ?? 0
    if (da !== db) return da - db
  }
  return 0
}

function formatDate(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso.slice(0, 10)
  const y = d.getUTCFullYear()
  const m = String(d.getUTCMonth() + 1).padStart(2, '0')
  const day = String(d.getUTCDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

export default defineLoader({
  watch: [],
  async load(): Promise<readonly PyPIRelease[]> {
    try {
      const response = await fetch(ENDPOINT, { headers: { Accept: 'application/json' } })
      if (!response.ok) {
        console.warn(`[data:pypi-releases:fetch] PyPI returned ${response.status}, using fallback`)
        return FALLBACK
      }
      const payload = await response.json() as PyPIPayload
      const entries: PyPIRelease[] = []
      for (const [version, files] of Object.entries(payload.releases)) {
        if (!files || files.length === 0) continue
        const live = files.find(f => !f.yanked) ?? files[0]
        entries.push({
          date    : formatDate(live.upload_time),
          url     : `https://pypi.org/project/${PACKAGE}/${version}/`,
          version
        })
      }
      entries.sort(compareDesc)
      return entries.length > 0 ? entries : FALLBACK
    } catch (error) {
      console.warn('[data:pypi-releases:fetch] external call failed, using fallback', error)
      return FALLBACK
    }
  }
})
