import loader              from '../../lib/reference/cli-flags.data'
import { flagRow }         from '../../lib/reference/cli-flags.data'
import { getRenderer }     from '../../lib/markdown/renderer'
import type { InlineNode } from '../../lib/markdown/inline-nodes'
import { nodeText }        from '../../lib/markdown/inline-text'

const flags = await loader.load([])
const md    = await getRenderer()

const text = (nodes: readonly InlineNode[]): string => nodes.map(nodeText).join('')

const keys = (table: keyof typeof flags): string[] => flags[table].map(row => row.key)

describe('cli flags', () => {
  it('leaves the global flags and the help builtins out of a subcommand table', () => {
    expect(keys('global')).toStrictEqual(['--color', '--verbose'])
    expect(keys('format')).toContain('--diff')
    expect(keys('format')).not.toContain('--color')
    expect(keys('check')).not.toContain('--help')
  })

  it('gives every row its help line, closed with a period', () => {
    for (const row of Object.values(flags).flat()) expect(text(row.meaningNodes)).toMatch(/\.$/)
  })

  it('names a short flag beside its long one', () => {
    expect(keys('check')).toContain('--quiet, -q')
  })
})

describe('flagRow', () => {
  const slugs = new Set(['a', 'b'])
  const type  = (entry: Parameters<typeof flagRow>[1]): string => text(flagRow(md, entry, slugs).typeNodes)

  it.each([
    [{ help: 'x', name: '--select', value: 'RULES', values: ['b', 'a'] }, 'rule slugs'],
    [{ help: 'x', name: '--color', value: 'WHEN', values: ['auto', 'never'] }, 'auto | never'],
    [{ help: 'x', name: '--stdin-filename', value: 'PATH' }, '<PATH>'],
    [{ help: 'x', name: '--diff' }, 'bool'],
    [{ help: 'x', name: 'PATH...' }, 'path']
  ])('types %j as %j', (entry, expected) => {
    expect(type(entry)).toBe(expected)
  })

  it('defaults a bool to off and a valued flag to unset', () => {
    expect(flagRow(md, { help: 'x', name: '--diff' }, slugs).default).toBe('off')
    expect(flagRow(md, { help: 'x', name: '--to', value: 'PATH' }, slugs).default).toBe('unset')
  })
})
