import { readPublishedVersion } from '../../../site/.vitepress/lib/shared/published-version'
import { supportTest }          from '../support'

const LATEST = 'https://api.github.com/repos/Jybbs/prose/releases/latest'

beforeEach(() => {
  vi.stubEnv('PROSE_OFFLINE_DOCS', '')
})

describe('readPublishedVersion', () => {
  supportTest('reads the tag of the latest release with the build token', async ({ tmpDir }) => {
    vi.stubEnv('GITHUB_TOKEN', 'ghs_token')
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response('{"draft":false,"tag_name":"0.11.0"}', { status: 200 })
    )
    vi.stubGlobal('fetch', fetchMock)
    await expect(readPublishedVersion(tmpDir)).resolves.toBe('0.11.0')
    expect(fetchMock).toHaveBeenCalledExactlyOnceWith(LATEST, expect.objectContaining({
      headers: expect.objectContaining({ Authorization: 'Bearer ghs_token' })
    }))
  })

  supportTest('returns no version on a 404 from a cold store', async ({ tmpDir, warn }) => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 404 })))
    await expect(readPublishedVersion(tmpDir)).resolves.toBeNull()
    expect(warn).toHaveBeenCalledWith('[data:published-version] upstream returned 404, seeding the fallback')
  })

  supportTest('returns no version on a release with no tag', async ({ tmpDir, warn }) => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(new Response('{}', { status: 200 })))
    await expect(readPublishedVersion(tmpDir)).resolves.toBeNull()
    expect(warn).toHaveBeenCalledWith(
      '[data:published-version] payload rejected (Could not find tag_name in the latest release), '
      + 'seeding the fallback'
    )
  })

  supportTest('returns no version when the request fails from a cold store', async ({ tmpDir, warn }) => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockRejectedValue(new TypeError('fetch failed')))
    await expect(readPublishedVersion(tmpDir)).resolves.toBeNull()
    expect(warn).toHaveBeenCalledWith('[data:published-version] request failed, seeding the fallback')
  })
})
