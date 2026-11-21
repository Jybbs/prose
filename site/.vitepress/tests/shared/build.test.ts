import loader from '../../lib/shared/build.data'

import type { walkFixtures }         from '../../lib/fixtures/walker'
import type { readPublishedVersion } from '../../lib/shared/published-version'

vi.mock('../../lib/fixtures/walker', () => ({ walkFixtures: vi.fn<typeof walkFixtures>(function* () {}) }))
vi.mock('../../lib/shared/published-version', () => ({
  readPublishedVersion: vi.fn<typeof readPublishedVersion>().mockResolvedValue('9.8.7')
}))

describe('the build loader', () => {
  it('reports the published release as the version', async () => {
    await expect(loader.load([])).resolves.toMatchObject({ fixtureCount: 0, version: '9.8.7' })
  })
})
