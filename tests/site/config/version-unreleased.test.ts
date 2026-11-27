import config       from '../../../site/.vitepress/config'
import { REPO_URL } from '../../../site/.vitepress/lib/shared/constants'

import type { PageData }             from 'vitepress'
import type { readPublishedVersion } from '../../../site/.vitepress/lib/shared/published-version'

vi.mock('../../../site/.vitepress/lib/shared/published-version', () => ({
  readPublishedVersion: vi.fn<typeof readPublishedVersion>().mockResolvedValue(null)
}))

function page(relativePath: string): PageData {
  return { description: 'd', frontmatter: {}, relativePath, title: 'Home' } as PageData
}

describe('the config with no published release', () => {
  it('labels the nav item as the releases page', () => {
    expect(config.themeConfig?.nav?.at(-1)).toStrictEqual({ link: `${REPO_URL}/releases`, text: 'Releases' })
  })

  it('leaves the version out of the landing graph', async () => {
    const tags   = await config.transformHead!({ pageData: page('index.md') } as never)
    const script = tags?.find(([tag]) => tag === 'script')
    expect(JSON.parse(script?.[2] as string)['@graph'][1]).not.toHaveProperty('softwareVersion')
  })

  it('hands no release to the source page dependency line', async () => {
    const data = page('architecture/source.md')
    await config.transformPageData!(data, { siteConfig: { srcDir: '' } } as never)
    expect(data.frontmatter.proseTag).toBe('')
  })
})
