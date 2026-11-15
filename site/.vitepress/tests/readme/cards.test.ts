import path from 'node:path'

import { CARDS, drawCards } from '../../lib/readme/cards'
import { SCHEMES }          from '../../lib/shared/palette'
import { readmeDir }        from '../../lib/shared/paths'

const DRAWN = await drawCards()

describe('drawCards', () => {
  it('draws every card in each scheme', () => {
    expect(Object.keys(DRAWN).sort()).toStrictEqual(
      CARDS.flatMap(card => SCHEMES.map(scheme => `${card.label.toLowerCase()}-${scheme}`)).sort())
  })

  it.each(Object.keys(DRAWN))('draws %s as outlines at the card size', name => {
    expect(DRAWN[name]).toMatch(/^<svg width="260" height="168"/)
    expect(DRAWN[name]).not.toContain('<text')
  })

  it.each(Object.keys(DRAWN))('matches the committed %s card', async name => {
    await expect(DRAWN[name]).toMatchFileSnapshot(path.join(readmeDir(import.meta.url), `${name}.svg`))
  })
})
