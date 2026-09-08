import fs   from 'node:fs'
import path from 'node:path'

import * as cacache from 'cacache'

import * as cache            from '../../lib/og/render/cache'
import { supportTest }       from '../support'

// A plain file where cacache expects a directory, which every store call fails against.
const blockedStore = (dir: string): string => {
  const file = path.join(dir, 'not-a-directory')
  fs.writeFileSync(file, '')
  return file
}

describe('readCard', () => {
  supportTest('reads back what writeCard stored', async ({ tmpDir }) => {
    await cache.writeCard(tmpDir, 'key', Buffer.from('PNG'))
    await expect(cache.readCard(tmpDir, 'key')).resolves.toStrictEqual(Buffer.from('PNG'))
  })

  supportTest('resolves null on a miss', async ({ tmpDir }) => {
    await expect(cache.readCard(tmpDir, 'absent')).resolves.toBeNull()
  })
})

describe('writeCard', () => {
  supportTest('swallows a store it cannot write', async ({ tmpDir }) => {
    await expect(cache.writeCard(blockedStore(tmpDir), 'key', Buffer.from('PNG')))
      .resolves.toBeUndefined()
  })
})

describe('pruneCards', () => {
  supportTest('drops the entries no live key names and keeps the rest', async ({ tmpDir }) => {
    await cache.writeCard(tmpDir, 'live', Buffer.from('A'))
    await cache.writeCard(tmpDir, 'stale', Buffer.from('B'))
    await cache.pruneCards(tmpDir, ['live'])
    expect(Object.keys(await cacache.ls(tmpDir))).toStrictEqual(['live'])
  })

  supportTest('leaves the store alone when every entry is live', async ({ tmpDir }) => {
    await cache.writeCard(tmpDir, 'live', Buffer.from('A'))
    await cache.pruneCards(tmpDir, ['live'])
    await expect(cache.readCard(tmpDir, 'live')).resolves.toStrictEqual(Buffer.from('A'))
  })

  supportTest('swallows a store it cannot list', async ({ tmpDir }) => {
    await expect(cache.pruneCards(blockedStore(tmpDir), [])).resolves.toBeUndefined()
  })
})
