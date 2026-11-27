// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { BuildMetadata } from '../../../site/.vitepress/theme/components/layout/BuildMetadata.tsx'

const build = vi.hoisted(() => ({ fixtureCount: 3, gitSha: 'abc1234', version: null as string | null }))

vi.mock('vitepress', () => ({ useData: () => ({ page: { value: { lastUpdated: 0 } } }) }))
vi.mock('../../../site/.vitepress/lib/shared/build.data', () => ({ data: build }))
vi.mock('../../../site/.vitepress/lib/rules/rules.data', () => ({ data: { list: [] } }))

function labels(): string[] {
  return mount(BuildMetadata).findAll('span.uppercase').map(label => label.text())
}

describe('BuildMetadata', () => {
  it('lists the version first where a release is known', () => {
    build.version = '0.11.0'
    expect(labels()[0]).toBe('Version')
  })

  it('leaves the version out where none is known', () => {
    build.version = null
    expect(labels()).not.toContain('Version')
  })
})
