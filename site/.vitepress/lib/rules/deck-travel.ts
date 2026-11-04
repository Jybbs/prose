import { millis, reducesMotion } from '../shared/paint'

const travels = new WeakMap<Element, Animation>()

// Moves each composition deck in `list` from where it stood before `change`
// to where the new state sets it, over the time and curve the card's body
// grows by, so a deck travels down with an opening card's edge and back up
// with a closing one's. A deck starts no lower than the card's foot as the
// new state first draws it, since a closing card's head can lose a line at
// once, and under reduced motion every deck takes its place at once.
export async function deckTravel(list: HTMLElement, change: () => Promise<void>): Promise<void> {
  const win    = list.ownerDocument.defaultView!
  const decks  = () => [...list.querySelectorAll<HTMLElement>(':scope > li.fixture-card .composition-cards-ticks')]
    .map(deck => [deck.closest<HTMLElement>('li.fixture-card')!, deck] as const)
  const lift   = (card: Element, deck: Element) => deck.getBoundingClientRect().top - card.getBoundingClientRect().top
  const before = new Map(decks().map(([card, deck]) => [card, lift(card, deck)]))

  await change()
  if (reducesMotion(win)) return

  for (const [card, deck] of decks()) {
    const start = before.get(card)
    if (start === undefined) continue
    travels.get(deck)?.cancel()
    const from = Math.min(start, card.clientHeight - deck.offsetHeight) - lift(card, deck)
    if (Math.abs(from) < 1) continue
    const style = win.getComputedStyle(card)
    travels.set(deck, deck.animate([{ transform: `translateY(${from}px)` }, { transform: 'none' }], {
      duration : millis(style.getPropertyValue('--card-grow')),
      easing   : style.getPropertyValue('--card-ease-morph').trim()
    }))
  }
}
