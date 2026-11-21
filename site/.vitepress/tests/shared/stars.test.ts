import { conditionalFetch } from '../../lib/shared/conditional-fetch'
import loader               from '../../lib/shared/stars.data'

vi.mock('../../lib/shared/conditional-fetch', () => ({
  conditionalFetch: vi.fn<typeof conditionalFetch>().mockResolvedValue(1234)
}))

describe('the stars loader', () => {
  it('reads the star count with the build token', async () => {
    vi.stubEnv('GITHUB_TOKEN', 'ghs_token')
    await expect(loader.load([])).resolves.toStrictEqual({ stars: '1.2k' })
    expect(conditionalFetch).toHaveBeenCalledWith(expect.objectContaining({
      headers: expect.objectContaining({ Authorization: 'Bearer ghs_token' })
    }))
  })
})
