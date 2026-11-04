// @vitest-environment happy-dom
import { nextTick, ref, shallowRef } from 'vue'

import { useGlideWash }                 from '../../lib/composables/use-glide-wash'
import { domTest, mountSetup, stubRect } from '../dom'

// Builds a grid holding a wash and two tiles, the second of them active.
function gridOf() {
  const grid = document.createElement('div')
  grid.innerHTML = '<span class="wash"></span><a class="tile" data-layer="base"></a><a class="tile is-active" data-layer="analysis"></a>'
  document.body.replaceChildren(grid)
  stubRect(grid, { left: 10, top: 20, width: 400 })
  const [first, second] = grid.querySelectorAll<HTMLElement>('.tile')
  stubRect(first!, { height: 30, left: 10, top: 20, width: 90 })
  stubRect(second!, { height: 30, left: 110, top: 60, width: 90 })
  return { grid, wash: grid.querySelector<HTMLElement>('.wash')! }
}

describe('useGlideWash', () => {
  it('places the wash behind the active tile, in place on its first placement', async () => {
    const { grid, wash } = gridOf()
    mountSetup(() => useGlideWash(shallowRef(grid), shallowRef(wash), '.is-active', ref(0)))
    await nextTick()
    expect(wash.style.transform).toBe('translate(100px, 40px)')
    expect([wash.style.width, wash.style.height]).toStrictEqual(['90px', '30px'])
    expect([wash.dataset.layer, wash.dataset.on, 'snap' in wash.dataset]).toStrictEqual(['analysis', '', false])
  })

  it('glides to the next active tile without snapping', async () => {
    const { grid, wash } = gridOf()
    const active = ref(0)
    mountSetup(() => useGlideWash(shallowRef(grid), shallowRef(wash), '.is-active', active))
    const snapped = vi.fn<MutationCallback>()
    new MutationObserver(snapped).observe(wash, { attributeFilter: ['data-snap'] })
    grid.querySelector('.is-active')!.classList.remove('is-active')
    grid.querySelector('.tile')!.classList.add('is-active')
    active.value += 1
    await nextTick()
    await nextTick()
    expect(wash.style.transform).toBe('translate(0px, 0px)')
    expect(wash.dataset.layer).toBe('base')
    expect(snapped).not.toHaveBeenCalled()
  })

  domTest('snaps the wash again once the grid\'s width changes', async ({ resizeObserver }) => {
    const { grid, wash } = gridOf()
    mountSetup(() => useGlideWash(shallowRef(grid), shallowRef(wash), '.is-active', ref(0)))
    stubRect(grid, { left: 10, top: 20, width: 300 })
    const snapped = vi.fn<MutationCallback>()
    new MutationObserver(snapped).observe(wash, { attributeFilter: ['data-snap'] })
    resizeObserver.fire()
    await nextTick()
    expect(snapped).toHaveBeenCalled()
  })

  it('places nothing where no tile is active', () => {
    const { grid, wash } = gridOf()
    grid.querySelector('.is-active')!.classList.remove('is-active')
    mountSetup(() => useGlideWash(shallowRef(grid), shallowRef(wash), '.is-active', ref(0)))
    expect('on' in wash.dataset).toBe(false)
  })
})
