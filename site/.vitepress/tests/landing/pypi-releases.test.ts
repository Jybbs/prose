import loader            from '../../lib/landing/pypi-releases.data'
import { fetchCacheDir } from '../../lib/shared/paths'
import { supportTest }   from '../support'

vi.mock('../../lib/shared/paths', async importOriginal => ({
  ...await importOriginal<typeof import('../../lib/shared/paths')>(),
  fetchCacheDir: vi.fn<typeof fetchCacheDir>()
}))

const respond = (releases: Record<string, readonly object[]>) =>
  vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(
    new Response(JSON.stringify({ releases }), { status: 200 })
  ))

beforeEach(() => {
  vi.stubEnv('PROSE_OFFLINE_DOCS', '')
})

describe('the PyPI releases loader', () => {
  supportTest('shows no release when PyPI is unreachable and the store is cold', async ({ tmpDir, warn }) => {
    vi.mocked(fetchCacheDir).mockReturnValue(tmpDir)
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockRejectedValue(new TypeError('fetch failed')))
    await expect(loader.load([])).resolves.toStrictEqual([])
    expect(warn).toHaveBeenCalledWith('[data:pypi-releases] request failed, seeding the fallback')
  })

  supportTest('shows no release when PyPI holds no file for any version', async ({ tmpDir }) => {
    vi.mocked(fetchCacheDir).mockReturnValue(tmpDir)
    respond({ '0.11.1': [] })
    await expect(loader.load([])).resolves.toStrictEqual([])
  })

  supportTest('lists releases newest first, dated by the first file not yanked', async ({ tmpDir }) => {
    vi.mocked(fetchCacheDir).mockReturnValue(tmpDir)
    respond({
      '0.9.0'  : [{ upload_time: '2026-03-02T10:00:00', yanked: true },
                  { upload_time: '2026-03-04T10:00:00' }],
      '0.10.0' : [{ upload_time: '2026-05-12T09:30:00' }],
      '0.11.1' : []
    })
    await expect(loader.load([])).resolves.toStrictEqual([
      {
        date      : '2026-05-12',
        month     : 'MAY',
        url       : 'https://pypi.org/project/prose-formatter/0.10.0/',
        version   : '0.10.0',
        year      : '2026',
        yearShort : '26'
      },
      {
        date      : '2026-03-04',
        month     : 'MAR',
        url       : 'https://pypi.org/project/prose-formatter/0.9.0/',
        version   : '0.9.0',
        year      : '2026',
        yearShort : '26'
      }
    ])
  })
})
