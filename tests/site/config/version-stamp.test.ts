import config       from '../../../site/.vitepress/config'
import { REPO_URL } from '../../../site/.vitepress/lib/shared/constants'

import type { PageData }             from 'vitepress'
import type { readPublishedVersion } from '../../../site/.vitepress/lib/shared/published-version'

vi.mock('../../../site/.vitepress/lib/shared/published-version', () => ({
  readPublishedVersion: vi.fn<typeof readPublishedVersion>().mockResolvedValue('9.8.7')
}))

function page(relativePath: string): PageData {
  return { description: 'd', frontmatter: {}, relativePath, title: 'Home' } as PageData
}

describe('the version the config prints', () => {
  it('stamps the nav with the published release', () => {
    expect(config.themeConfig?.nav?.at(-1)).toStrictEqual({ link: `${REPO_URL}/releases`, text: 'v9.8.7' })
  })

  it('pins the landing graph to the published release', async () => {
    const tags   = await config.transformHead!({ pageData: page('index.md') } as never)
    const script = tags?.find(([tag]) => tag === 'script')
    expect(JSON.parse(script?.[2] as string)['@graph'][1]).toMatchObject({ softwareVersion: '9.8.7' })
  })

  it('hands the published release to the source page dependency line', async () => {
    const data = page('architecture/source.md')
    await config.transformPageData!(data, { siteConfig: { srcDir: '' } } as never)
    expect(data.frontmatter.proseVersion).toBe('9.8.7')
  })
})
