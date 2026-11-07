// @vitest-environment happy-dom
import { deckTravel } from '../../lib/rules/deck-travel'
import { domTest }    from '../dom'

// Builds a list of one card holding a deck the card's rect and the deck's
// top place, the card growing over `--prose-duration-morph` on its morph curve.
function listOf(): { card: HTMLElement, deck: HTMLElement, list: HTMLElement } {
  const list = document.createElement('ol')
  list.innerHTML = '<li class="fixture-card" style="--prose-duration-morph: 0.4s; --prose-ease-morph: ease"><ol class="composition-cards-ticks"></ol></li>'
  document.body.replaceChildren(list)
  const card = list.querySelector<HTMLElement>('li')!
  return { card, deck: card.querySelector<HTMLElement>('ol')!, list }
}

const at = (el: HTMLElement, top: number) => {
  el.getBoundingClientRect = () => ({ top }) as DOMRect
}

describe('deckTravel', () => {
  domTest('moves a deck from where it stood before the change, over the card\'s growth', async () => {
    const { card, deck, list } = listOf()
    at(card, 0)
    at(deck, 10)
    Object.defineProperty(card, 'clientHeight', { value: 300 })
    const animate = vi.spyOn(deck, 'animate')
    await deckTravel(list, () => Promise.resolve(at(deck, 200)))
    expect(animate).toHaveBeenCalledWith(
      [{ transform: 'translateY(-190px)' }, { transform: 'none' }],
      { duration: 400, easing: 'ease' }
    )
  })

  domTest('starts a deck no lower than the card\'s foot', async () => {
    const { card, deck, list } = listOf()
    at(card, 0)
    at(deck, 200)
    Object.defineProperty(card, 'clientHeight', { value: 60 })
    Object.defineProperty(deck, 'offsetHeight', { value: 20 })
    const animate = vi.spyOn(deck, 'animate')
    await deckTravel(list, () => Promise.resolve(at(deck, 10)))
    expect(animate.mock.calls[0]![0]).toStrictEqual([{ transform: 'translateY(30px)' }, { transform: 'none' }])
  })

  domTest('leaves the deck of a card the change adds where it is drawn', async () => {
    const list = document.createElement('ol')
    document.body.replaceChildren(list)
    await deckTravel(list, () => {
      list.innerHTML = '<li class="fixture-card"><ol class="composition-cards-ticks"></ol></li>'
      return Promise.resolve()
    })
    expect(list.querySelector('ol')!.getAnimations()).toStrictEqual([])
  })

  domTest('leaves a deck the change did not move, and every deck under reduced motion', async ({ reducedMotion }) => {
    const { card, deck, list } = listOf()
    at(card, 0)
    at(deck, 10)
    Object.defineProperty(card, 'clientHeight', { value: 300 })
    const animate = vi.spyOn(deck, 'animate')
    await deckTravel(list, () => Promise.resolve())
    reducedMotion(true)
    await deckTravel(list, () => Promise.resolve(at(deck, 200)))
    expect(animate).not.toHaveBeenCalled()
  })
})
