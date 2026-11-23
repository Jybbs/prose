// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import type { PyPIRelease } from '../../lib/landing/pypi-releases.data'
import { Cta }              from '../../theme/components/landing/Cta'

const releases = vi.hoisted((): PyPIRelease[] => [])

vi.mock('../../lib/landing/pypi-releases.data', () => ({ data: releases }))

afterEach(() => {
  releases.length = 0
})

describe('Cta', () => {
  it('stamps the newest release with a link to its PyPI page', () => {
    releases.push({
      date      : '2026-05-12',
      month     : 'MAY',
      url       : 'https://pypi.org/project/prose-formatter/0.10.0/',
      version   : '0.10.0',
      year      : '2026',
      yearShort : '26'
    })
    const w = mount(Cta)
    expect(w.find('a[href^="https://pypi.org/"]').text()).toContain('0.10.0')
    expect(w.find('[aria-label="Show previous releases"]').exists()).toBe(true)
  })

  it('links to no release and offers no ledger where none is known', () => {
    const w = mount(Cta)
    expect(w.find('a[href^="https://pypi.org/"]').exists()).toBe(false)
    expect(w.find('[aria-label="Show previous releases"]').exists()).toBe(false)
  })
})
