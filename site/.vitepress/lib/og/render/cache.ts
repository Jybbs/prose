import * as cacache from 'cacache'

export async function pruneCards(cacheDir: string, live: Iterable<string>): Promise<void> {
  try {
    const keep  = new Set(live)
    const index = await cacache.ls(cacheDir)
    const stale = Object.keys(index).filter(key => !keep.has(key))
    if (stale.length === 0) return
    await Promise.all(stale.map(key => cacache.rm.entry(cacheDir, key)))
    await cacache.verify(cacheDir)
  }
  catch {
    // Prune is best-effort housekeeping
  }
}

export async function readCard(cacheDir: string, key: string): Promise<Buffer | null> {
  try {
    return (await cacache.get(cacheDir, key)).data
  }
  catch {
    // A miss or a failed integrity check both fall through to a fresh render
    return null
  }
}

export async function writeCard(cacheDir: string, key: string, png: Buffer): Promise<void> {
  try {
    await cacache.put(cacheDir, key, png)
  }
  catch {
    // A failed write still leaves the rendered card in dist
  }
}
