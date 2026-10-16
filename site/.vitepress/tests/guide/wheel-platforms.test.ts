// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import WheelPlatforms from '../../theme/components/guide/WheelPlatforms.vue'

vi.mock('../../lib/guide/platforms.data', () => ({
  data: [
    { emoji: '🐧', label: 'Linux x86_64',  manylinux: true,  name: 'Linux', target: 'x86_64-unknown-linux-gnu' },
    { emoji: '🍎', label: 'macOS aarch64', manylinux: false, name: 'macOS', target: 'aarch64-apple-darwin' }
  ]
}))

const rows = () => mount(WheelPlatforms).findAll('tbody tr')

describe('WheelPlatforms', () => {
  it('opens each label on its platform badge in a running-text chip', () => {
    expect(rows().map(row => [
      row.get('.nib-chip use').attributes('href'),
      row.findAll('td')[1].text()
    ])).toStrictEqual([
      ['#nib-penguin-small', 'Linux x86_64 (glibc, manylinux)'],
      ['#nib-apple-small',   'macOS aarch64']
    ])
  })

  it('gives the platform chip no family, so it takes Ube', () => {
    expect(rows()[0].get('.nib-chip').attributes('data-family')).toBeUndefined()
  })
})
