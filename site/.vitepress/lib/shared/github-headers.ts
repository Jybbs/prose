import { USER_AGENT } from './constants'

// Builds the headers of a GitHub REST request, adding the build's
// `GITHUB_TOKEN` as a bearer token where the environment sets one.
export function githubHeaders(): Record<string, string> {
  const token = process.env.GITHUB_TOKEN
  return {
    Accept       : 'application/vnd.github+json',
    'User-Agent' : USER_AGENT,
    ...(token ? { Authorization: `Bearer ${token}` } : {})
  }
}
