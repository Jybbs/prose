import { conditionalFetch } from './conditional-fetch.ts'
import { REPO_SLUG }        from './constants.ts'
import { githubHeaders }    from './github-headers.ts'
import { requireString }    from './require-string.ts'
import { readCargoVersion } from './version.ts'

// Reads the tag of the newest published release from GitHub's
// `releases/latest`, which leaves out every draft and prerelease,
// authenticating where the build sets `GITHUB_TOKEN` and falling back to the
// manifest's version where a cold store holds no answer.
export function readPublishedVersion(crateDir: string, cacheDir: string): Promise<string> {
  return conditionalFetch({
    dir      : cacheDir,
    fallback : readCargoVersion(crateDir),
    headers  : githubHeaders(),
    key      : 'published-version',
    parse    : payload => requireString(
      (payload as { tag_name?: unknown }).tag_name,
      'Could not find tag_name in the latest release'
    ),
    url      : `https://api.github.com/repos/${REPO_SLUG}/releases/latest`
  })
}
