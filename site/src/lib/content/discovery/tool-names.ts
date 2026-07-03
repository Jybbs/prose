import { getCollection } from 'astro:content'

import { lazy }     from '../../shared/lazy'
import { required } from '../../shared/required'

const toolNames = lazy(async () =>
  new Map((await getCollection('tools')).map(tool => [tool.id, tool.data.name]))
)

// Resolves a tool id to its display name, `noun` naming the referrer in the
// build-time error a missing id raises.
export async function toolNamer(noun: string): Promise<(id: string) => string> {
  const names = await toolNames()
  return id => required(names.get(id), `${noun} "${id}" is not in the tools collection`)
}
