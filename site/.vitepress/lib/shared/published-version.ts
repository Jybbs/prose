import { conditionalFetch } from './conditional-fetch.ts'
import { REPO_SLUG }        from './constants.ts'
import { githubHeaders }    from './github-headers.ts'
import { requireString }    from './require-string.ts'

// Reads the tag of the newest published release from GitHub's
// `releases/latest`, which leaves out every draft and prerelease,
// authenticating where the build sets `GITHUB_TOKEN` and returning `null`
// where a cold store holds no answer, since the manifest's version may name
// a release that is not yet published.
export function readPublishedVersion(cacheDir: string): Promise<string | null> {
  return conditionalFetch({
    dir      : cacheDir,
    fallback : null,
    headers  : githubHeaders(),
    key      : 'published-version',
    parse    : (payload): string | null => requireString(
      (payload as { tag_name?: unknown }).tag_name,
      'Could not find tag_name in the latest release'
    ),
    url      : `https://api.github.com/repos/${REPO_SLUG}/releases/latest`
  })
}
