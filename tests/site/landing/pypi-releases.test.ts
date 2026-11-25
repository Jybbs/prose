import loader            from '../../../site/.vitepress/lib/landing/pypi-releases.data.ts'
import { fetchCacheDir } from '../../../site/.vitepress/lib/shared/paths.ts'
import { supportTest }   from '../support.ts'

vi.mock('../../../site/.vitepress/lib/shared/paths', async importOriginal => ({
  ...await importOriginal<typeof import('../../../site/.vitepress/lib/shared/paths.ts')>(),
  fetchCacheDir: vi.fn<typeof fetchCacheDir>()
}))

const loadFrom = (dir: string) => {
  vi.mocked(fetchCacheDir).mockReturnValue(dir)
  return loader.load([])
}

const respond = (payload: object) =>
  vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(Response.json(payload)))

beforeEach(() => {
  vi.stubEnv('PROSE_OFFLINE_DOCS', '')
})

describe('the PyPI releases loader', () => {
  supportTest('returns no release when PyPI is unreachable and the store is cold',
    async ({ tmpDir, warn }) => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockRejectedValue(new TypeError('fetch failed')))
    await expect(loadFrom(tmpDir)).resolves.toStrictEqual([])
    expect(warn).toHaveBeenCalledWith('[data:pypi-releases] request failed, seeding the fallback')
  })

  supportTest('returns no release when PyPI holds no file for any version', async ({ tmpDir, warn }) => {
    respond({ releases: { '0.11.1': [] } })
    await expect(loadFrom(tmpDir)).resolves.toStrictEqual([])
    expect(warn).not.toHaveBeenCalled()
  })

  supportTest.for([
    ['the release map is missing', {}],
    ['a version has no file list', { releases: { '0.9.0': null } }],
    ['a file has no upload time', { releases: { '0.9.0': [{}] } }],
    ['an upload time is unreadable', { releases: { '0.9.0': [{ upload_time: 'soon' }] } }]
  ] as const)('returns no release on a cold store when %s', async ([, payload], { tmpDir, warn }) => {
    respond(payload)
    await expect(loadFrom(tmpDir)).resolves.toStrictEqual([])
    expect(warn).toHaveBeenCalledWith(expect.stringContaining('payload rejected'))
  })

  supportTest('orders releases uploaded on the same day by version number, highest first',
    async ({ tmpDir }) => {
    respond({ releases: {
      '0.9.0'  : [{ upload_time: '2026-05-12T08:00:00' }],
      '0.10.0' : [{ upload_time: '2026-05-12T09:30:00' }]
    } })
    await expect(loadFrom(tmpDir)).resolves.toMatchObject([{ version: '0.10.0' }, { version: '0.9.0' }])
  })

  supportTest('lists releases newest first, each dated by its first file not yanked or else its first file',
    async ({ tmpDir }) => {
    respond({ releases: {
      '0.10.0' : [{ upload_time: '2026-05-12T09:30:00' }],
      '0.11.1' : [],

      '0.8.0': [
        { upload_time: '2026-01-05T08:00:00', yanked: true },
        { upload_time: '2026-01-06T08:00:00', yanked: true }
      ],

      '0.9.0': [
        { upload_time: '2026-06-02T10:00:00', yanked: true },
        { upload_time: '2026-06-04T10:00:00' },
        { upload_time: '2026-06-06T10:00:00' }
      ]
    } })
    await expect(loadFrom(tmpDir)).resolves.toStrictEqual([
      {
        date      : '2026-06-04',
        month     : 'JUN',
        url       : 'https://pypi.org/project/prose-formatter/0.9.0/',
        version   : '0.9.0',
        year      : '2026',
        yearShort : '26'
      },
      {
        date      : '2026-05-12',
        month     : 'MAY',
        url       : 'https://pypi.org/project/prose-formatter/0.10.0/',
        version   : '0.10.0',
        year      : '2026',
        yearShort : '26'
      },
      {
        date      : '2026-01-05',
        month     : 'JAN',
        url       : 'https://pypi.org/project/prose-formatter/0.8.0/',
        version   : '0.8.0',
        year      : '2026',
        yearShort : '26'
      }
    ])
  })
})
