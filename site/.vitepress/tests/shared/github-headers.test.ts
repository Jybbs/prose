import { githubHeaders } from '../../lib/shared/github-headers'

const BASE = { Accept: 'application/vnd.github+json', 'User-Agent': 'prose-docs-build' }

describe('githubHeaders', () => {
  it.each([
    { expected: { ...BASE, Authorization: 'Bearer ghs_token' }, token: 'ghs_token' },
    { expected: BASE, token: '' }
  ])('sends the bearer token the environment sets, given $token', ({ expected, token }) => {
    vi.stubEnv('GITHUB_TOKEN', token)
    expect(githubHeaders()).toStrictEqual(expected)
  })
})
