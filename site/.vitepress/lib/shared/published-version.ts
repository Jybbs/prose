import { conditionalFetch }      from './conditional-fetch'
import { REPO_SLUG, USER_AGENT } from './constants'
import { requireString }         from './require-string'
import { readCargoVersion }      from './version'

// Reads the tag of the newest published release from GitHub's
// `releases/latest`, which leaves out every draft and prerelease, falling
// back to the manifest's version where a cold store holds no answer.
export function readPublishedVersion(crateDir: string, cacheDir: string): Promise<string> {
  return conditionalFetch({
    dir      : cacheDir,
    fallback : readCargoVersion(crateDir),
    headers  : { Accept: 'application/vnd.github+json', 'User-Agent': USER_AGENT },
    key      : 'published-version',
    parse    : payload => requireString(
      (payload as { tag_name?: unknown }).tag_name,
      'Could not find tag_name in the latest release'
    ),
    url      : `https://api.github.com/repos/${REPO_SLUG}/releases/latest`
  })
}
