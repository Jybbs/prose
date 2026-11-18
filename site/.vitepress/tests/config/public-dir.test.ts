import { resolveConfig } from 'vitepress'

import { publicDir, siteDir } from '../../lib/shared/paths'

const resolved = await resolveConfig(siteDir(import.meta.url), 'build', 'production')

describe('the public folder', () => {
  it('resolves to `site/public` rather than the folder under `srcDir`', () => {
    expect(resolved.publicDir).toBe(publicDir(import.meta.url))
  })
})
