import { glob, rmdir } from 'node:fs/promises'
import path            from 'node:path'

import * as cacache from 'cacache'

// Declares the `removeFully` option cacache 21 documents for `rm.entry`, which
// @types/cacache 20 leaves out of the signature.
declare module 'cacache' {
  namespace rm {
    function entry(cachePath: string, key: string, opts: { removeFully: boolean }): Promise<void>
  }
}

export async function pruneEntries(cacheDir: string, live: Iterable<string>): Promise<void> {
  try {
    const keep  = new Set(live)
    const index = await cacache.ls(cacheDir)
    const stale = Object.keys(index).filter(key => !keep.has(key))
    if (stale.length === 0) return
    // Deletes each stale key's index bucket through `removeFully`, where the
    // default appends a deletion entry that every later `ls` still reads.
    await Promise.all(stale.map(key => cacache.rm.entry(cacheDir, key, { removeFully: true })))
    await cacache.verify(cacheDir)
    await removeEmptyBuckets(cacheDir)
  }
  catch {
    // Prune is best-effort housekeeping
  }
}

// Removes each bucket directory a `removeFully` deletion leaves empty. cacache
// files a bucket two directories deep under its `index-*` directory, and `rmdir`
// refuses any directory that still holds one.
async function removeEmptyBuckets(cacheDir: string): Promise<void> {
  for (const pattern of ['index-*/*/*', 'index-*/*']) {
    const dirs = await Array.fromAsync(glob(pattern, { cwd: cacheDir }))
    await Promise.allSettled(dirs.map(dir => rmdir(path.join(cacheDir, dir))))
  }
}

export async function readEntry(cacheDir: string, key: string): Promise<Buffer | null> {
  try {
    return (await cacache.get(cacheDir, key)).data
  }
  catch {
    // A miss or a failed integrity check both fall through to a fresh render
    return null
  }
}

// Reads every entry the store under `cacheDir` holds, resolving an empty map
// where the store is missing or fails to read.
export async function readEntries(cacheDir: string): Promise<Map<string, Buffer>> {
  try {
    const index = Object.entries(await cacache.ls(cacheDir))
    return new Map(await Promise.all(index.map(async ([key, { integrity }]) => [key, await cacache.get.byDigest(cacheDir, integrity)] as const)))
  }
  catch {
    // A store that fails to read leaves every block to a fresh highlight
    return new Map()
  }
}

export async function writeEntry(cacheDir: string, key: string, data: Buffer | string): Promise<void> {
  try {
    await cacache.put(cacheDir, key, data)
  }
  catch {
    // A failed write still leaves the caller its rendered value
  }
}
