import fs   from 'node:fs'
import path from 'node:path'

import * as cacache from 'cacache'

import * as cache            from '../../lib/og/render/cache'
import type { BrandAssets }  from '../../lib/og/render/assets'
import { supportTest }       from '../support'
import { page as ruleCool }  from './cards/rule-cool/page'
import { page as plainPage } from './cards/plain-usage/page'

const BRAND: BrandAssets = {
  fonts            : [],
  glyph            : 'data:image/svg+xml;base64,Z2x5cGg=',
  titleWithTagline : { aspect: 4, src: 'data:image/svg+xml;base64,dGl0bGU=' },
  wordmark         : { aspect: 3, src: 'data:image/svg+xml;base64,bWFyaw==' }
}

describe('cardKeyer', () => {
  it('keys the same card the same way twice', () => {
    const keyer = cache.cardKeyer('0.9.0', BRAND)
    expect(keyer(ruleCool)).toBe(keyer(ruleCool))
  })

  it.each([
    ['another rule card', plainPage],
    ['the landing card',  'landing' as const]
  ])('separates a rule card from %s', (_case, other) => {
    const keyer = cache.cardKeyer('0.9.0', BRAND)
    expect(keyer(ruleCool)).not.toBe(keyer(other))
  })

  it('separates one version from the next', () => {
    expect(cache.cardKeyer('0.9.0', BRAND)(ruleCool))
      .not.toBe(cache.cardKeyer('0.9.1', BRAND)(ruleCool))
  })
})

describe('readCard', () => {
  supportTest('reads back what writeCard stored', async ({ tmpDir }) => {
    await cache.writeCard(tmpDir, 'key', Buffer.from('PNG'))
    await expect(cache.readCard(tmpDir, 'key')).resolves.toEqual(Buffer.from('PNG'))
  })

  supportTest('resolves null on a miss', async ({ tmpDir }) => {
    await expect(cache.readCard(tmpDir, 'absent')).resolves.toBeNull()
  })
})

describe('writeCard', () => {
  supportTest('swallows a store it cannot write', async ({ tmpDir }) => {
    const file = path.join(tmpDir, 'not-a-directory')
    fs.writeFileSync(file, '')
    await expect(cache.writeCard(file, 'key', Buffer.from('PNG'))).resolves.toBeUndefined()
  })
})

describe('pruneCards', () => {
  supportTest('drops the entries no live key names and keeps the rest', async ({ tmpDir }) => {
    await cache.writeCard(tmpDir, 'live', Buffer.from('A'))
    await cache.writeCard(tmpDir, 'stale', Buffer.from('B'))
    await cache.pruneCards(tmpDir, ['live'])
    expect(Object.keys(await cacache.ls(tmpDir))).toEqual(['live'])
  })

  supportTest('leaves the store alone when every entry is live', async ({ tmpDir }) => {
    await cache.writeCard(tmpDir, 'live', Buffer.from('A'))
    await cache.pruneCards(tmpDir, ['live'])
    await expect(cache.readCard(tmpDir, 'live')).resolves.toEqual(Buffer.from('A'))
  })

  supportTest('swallows a store it cannot list', async ({ tmpDir }) => {
    const file = path.join(tmpDir, 'not-a-directory')
    fs.writeFileSync(file, '')
    await expect(cache.pruneCards(file, [])).resolves.toBeUndefined()
  })
})
