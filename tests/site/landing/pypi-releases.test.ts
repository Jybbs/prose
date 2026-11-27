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

  supportTest('returns no release when PyPI lists no file', async ({ tmpDir, warn }) => {
    respond({ files: [] })
    await expect(loadFrom(tmpDir)).resolves.toStrictEqual([])
    expect(warn).not.toHaveBeenCalled()
  })

  supportTest.for([
    ['the file list is missing', {}],
    ['a file has no upload time', { files: [{ filename: 'prose_formatter-0.9.0.tar.gz' }] }],
    ['an upload time is unreadable',
      { files: [{ filename: 'prose_formatter-0.9.0.tar.gz', 'upload-time': 'soon' }] }]
  ] as const)('returns no release on a cold store when %s', async ([, payload], { tmpDir, warn }) => {
    respond(payload)
    await expect(loadFrom(tmpDir)).resolves.toStrictEqual([])
    expect(warn).toHaveBeenCalledWith(expect.stringContaining('payload rejected'))
  })

  supportTest('requests the Index API project detail', async ({ tmpDir }) => {
    respond({ files: [] })
    await loadFrom(tmpDir)
    expect(fetch).toHaveBeenCalledWith(
      'https://pypi.org/simple/prose-formatter/',
      expect.objectContaining({ headers: expect.objectContaining({ Accept: 'application/vnd.pypi.simple.v1+json' }) })
    )
  })

  supportTest('orders releases uploaded on the same day by version number, highest first',
    async ({ tmpDir }) => {
    respond({ files: [
      { filename: 'prose_formatter-0.9.0.tar.gz',  'upload-time': '2026-05-12T08:00:00Z' },
      { filename: 'prose_formatter-0.10.0.tar.gz', 'upload-time': '2026-05-12T09:30:00Z' }
    ] })
    await expect(loadFrom(tmpDir)).resolves.toMatchObject([{ version: '0.10.0' }, { version: '0.9.0' }])
  })

  supportTest('leaves out a release whose every file is yanked and keeps one with a file left',
    async ({ tmpDir }) => {
    respond({ files: [
      { filename: 'prose_formatter-0.10.0.tar.gz', 'upload-time': '2026-05-12T09:30:00Z', yanked: false },

      { filename: 'prose_formatter-0.8.0-py3-none-any.whl', 'upload-time': '2026-01-05T08:00:00Z', yanked: true },
      { filename: 'prose_formatter-0.8.0.tar.gz',           'upload-time': '2026-01-06T08:00:00Z', yanked: 'broken' },

      { filename: 'prose_formatter-0.9.0-py3-none-any.whl', 'upload-time': '2026-06-02T10:00:00Z', yanked: 'broken' },
      { filename: 'prose_formatter-0.9.0-py3-none-musl.whl', 'upload-time': '2026-06-04T10:00:00Z' },
      { filename: 'prose_formatter-0.9.0.tar.gz',            'upload-time': '2026-06-06T10:00:00Z', yanked: false }
    ] })
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
      }
    ])
  })
})
