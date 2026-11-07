import fs   from 'node:fs'
import path from 'node:path'

import * as cacache from 'cacache'

import * as cache            from '../../lib/shared/cache'
import { supportTest }       from '../support'

// A plain file where cacache expects a directory, which every store call fails against.
const blockedStore = (dir: string): string => {
  const file = path.join(dir, 'not-a-directory')
  fs.writeFileSync(file, '')
  return file
}

// Counts the bucket files under the store's `index-*` directory and the
// directories there that hold nothing.
const indexTree = (dir: string): { buckets: number, empty: number } => {
  const entries = fs.globSync('index-*/**/*', { cwd: dir, withFileTypes: true })
  return {
    buckets : entries.filter(entry => entry.isFile()).length,
    empty   : entries.filter(entry => entry.isDirectory()
      && fs.readdirSync(path.join(entry.parentPath, entry.name)).length === 0).length
  }
}

describe('readEntry', () => {
  supportTest('reads back what writeEntry stored', async ({ tmpDir }) => {
    await cache.writeEntry(tmpDir, 'key', Buffer.from('PNG'))
    await expect(cache.readEntry(tmpDir, 'key')).resolves.toStrictEqual(Buffer.from('PNG'))
  })

  supportTest('resolves null on a miss', async ({ tmpDir }) => {
    await expect(cache.readEntry(tmpDir, 'absent')).resolves.toBeNull()
  })
})

describe('readEntries', () => {
  supportTest('reads every entry the store holds', async ({ tmpDir }) => {
    await cache.writeEntry(tmpDir, 'a', 'A')
    await cache.writeEntry(tmpDir, 'b', Buffer.from('B'))
    await expect(cache.readEntries(tmpDir)).resolves.toStrictEqual(new Map([['a', Buffer.from('A')], ['b', Buffer.from('B')]]))
  })

  supportTest('resolves an empty map from a store it cannot read', async ({ tmpDir }) => {
    await expect(cache.readEntries(blockedStore(tmpDir))).resolves.toStrictEqual(new Map())
  })
})

describe('writeEntry', () => {
  supportTest('swallows a store it cannot write', async ({ tmpDir }) => {
    await expect(cache.writeEntry(blockedStore(tmpDir), 'key', Buffer.from('PNG')))
      .resolves.toBeUndefined()
  })
})

describe('pruneEntries', () => {
  supportTest('drops the entries no live key names and keeps the rest', async ({ tmpDir }) => {
    await cache.writeEntry(tmpDir, 'live', Buffer.from('A'))
    await cache.writeEntry(tmpDir, 'stale', Buffer.from('B'))
    await cache.pruneEntries(tmpDir, ['live'])
    expect(Object.keys(await cacache.ls(tmpDir))).toStrictEqual(['live'])
  })

  supportTest('drops a pruned key\'s bucket and reads the live entry back', async ({ tmpDir }) => {
    await cache.writeEntry(tmpDir, 'live', Buffer.from('A'))
    await cache.writeEntry(tmpDir, 'stale', Buffer.from('B'))
    await cache.pruneEntries(tmpDir, ['live'])
    expect(indexTree(tmpDir)).toStrictEqual({ buckets: 1, empty: 0 })
    await expect(cache.readEntry(tmpDir, 'live')).resolves.toStrictEqual(Buffer.from('A'))
  })

  supportTest('leaves the store alone when every entry is live', async ({ tmpDir }) => {
    await cache.writeEntry(tmpDir, 'live', Buffer.from('A'))
    await cache.pruneEntries(tmpDir, ['live'])
    await expect(cache.readEntry(tmpDir, 'live')).resolves.toStrictEqual(Buffer.from('A'))
  })

  supportTest('swallows a store it cannot list', async ({ tmpDir }) => {
    await expect(cache.pruneEntries(blockedStore(tmpDir), [])).resolves.toBeUndefined()
  })
})
