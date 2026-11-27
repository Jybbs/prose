import loader                  from '../../../site/.vitepress/lib/shared/build.data'
import { readPublishedVersion } from '../../../site/.vitepress/lib/shared/published-version'

import type { walkFixtures }         from '../../../site/.vitepress/lib/fixtures/walker'

vi.mock('../../../site/.vitepress/lib/fixtures/walker', () => ({ walkFixtures: vi.fn<typeof walkFixtures>(function* () {}) }))
vi.mock('../../../site/.vitepress/lib/shared/published-version', () => ({
  readPublishedVersion: vi.fn<typeof readPublishedVersion>().mockResolvedValue('9.8.7')
}))

describe('the build loader', () => {
  it('reports the published release as the version', async () => {
    await expect(loader.load([])).resolves.toMatchObject({ fixtureCount: 0, version: '9.8.7' })
  })

  it('reports no version where no release is known', async () => {
    vi.mocked(readPublishedVersion).mockResolvedValueOnce(null)
    await expect(loader.load([])).resolves.toMatchObject({ version: null })
  })
})
