import type { LoaderContext } from 'astro/loaders'

export type StoreEntry = { data: Record<string, unknown>, id: string }

// Clears a collection store and repopulates it from raw entries, parsing each
// through the loader context so the collection schema validates every record.
export async function replaceStore(
  ctx: Pick<LoaderContext, 'parseData' | 'store'>,
  entries: Iterable<StoreEntry>
): Promise<void> {
  ctx.store.clear()
  for (const { data, id } of entries) {
    ctx.store.set({ data: await ctx.parseData({ data, id }), id })
  }
}
