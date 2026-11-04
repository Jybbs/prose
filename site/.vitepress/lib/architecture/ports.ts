export type Port = 'both' | 'in' | 'out'

// Names the word each tab reads, which the stylesheet sets in capitals.
export const PORT_WORDS: Record<Port, string> = {
  both : 'in · out',
  in   : 'in',
  out  : 'out'
}

// Names the tab each tile related to the focused primitive takes, `in` for a
// tile it consumes, `out` for one that consumes it, and `both` for a tile
// that does each.
export function portsOf(entry: { consumedBy: readonly string[], consumes: readonly string[] } | null): Map<string, Port> {
  const ports = new Map<string, Port>()
  for (const slug of entry?.consumes ?? []) ports.set(slug, 'in')
  for (const slug of entry?.consumedBy ?? []) ports.set(slug, ports.has(slug) ? 'both' : 'out')
  return ports
}
