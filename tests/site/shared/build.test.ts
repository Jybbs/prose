import loader from '../../../site/.vitepress/lib/shared/build.data'

import type { walkFixtures }         from '../../../site/.vitepress/lib/fixtures/walker'
import type { readPublishedVersion } from '../../../site/.vitepress/lib/shared/published-version'

vi.mock('../../../site/.vitepress/lib/fixtures/walker', () => ({ walkFixtures: vi.fn<typeof walkFixtures>(function* () {}) }))
vi.mock('../../../site/.vitepress/lib/shared/published-version', () => ({
  readPublishedVersion: vi.fn<typeof readPublishedVersion>().mockResolvedValue('9.8.7')
}))

describe('the build loader', () => {
  it('reports the published release as the version', async () => {
    await expect(loader.load([])).resolves.toMatchObject({ fixtureCount: 0, version: '9.8.7' })
  })
})
