// @vitest-environment happy-dom
import path from 'node:path'

import { mount } from '@vue/test-utils'

import { expectAccessible }                 from '../axe'
import { componentFixtures, loadComponent } from '../fixture-tree'

describe('component fixtures', () => {
  it.each(componentFixtures())('$id', async fixture => {
    const w = mount(await loadComponent(fixture.component), {
      props : fixture.props,
      slots : fixture.slots
    })
    await expect(w.html()).toMatchFileSnapshot(path.join(fixture.dir, 'output.html.snap'))
    await expectAccessible(w.html(), fixture.axeIgnore)
  })

  it('fails loud on a component path outside the tree', async () => {
    await expect(loadComponent('base/Missing')).rejects.toThrow(/no component at theme\/components/)
  })
})
