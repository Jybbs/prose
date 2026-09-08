// oxlint-disable no-underscore-dangle -- VitePress names the field this plugin sets
import type { Plugin } from 'vite'

import { fixtureReloadPlugin } from '../../lib/fixtures/reload-plugin'

const ROOT = '/repo/crate/tests/fixtures'

interface Node {
  file          : string
  invalidated  ?: true
}

function harness(resolved = true) {
  const nodes: Node[] = [
    { file: '/repo/site/rules/alignment/align-equals.md' },
    { file: '/repo/site/.vitepress/theme/index.ts' }
  ]
  const sent    : unknown[] = []
  const watched : string[]  = []
  let fire: (event: string, file: string) => void = () => {}

  const server = {
    environments: {
      client: {
        hot         : { send: (payload: unknown) => sent.push(payload) },
        moduleGraph : {
          idToModuleMap    : new Map(nodes.map(node => [node.file, node])),
          invalidateModule : (node: Node) => { node.invalidated = true }
        }
      }
    },
    watcher: {
      add : (dir: string) => watched.push(dir),
      on  : (_event: string, handler: (event: string, file: string) => void) => { fire = handler }
    }
  }

  const site   = { __dirty: false }
  const plugin = fixtureReloadPlugin(ROOT) as Plugin & {
    configResolved  : (config: unknown) => void
    configureServer : (server: unknown) => void
  }
  if (resolved) plugin.configResolved({ vitepress: site })
  plugin.configureServer(server)
  return { fire: (file: string) => fire('change', file), nodes, sent, site, watched }
}

describe('fixtureReloadPlugin', () => {
  it('serves the dev server alone', () => {
    expect(fixtureReloadPlugin(ROOT).apply).toBe('serve')
  })

  it('watches the fixture tree, which sits outside the Vite root', () => {
    expect(harness().watched).toStrictEqual([ROOT])
  })

  it('invalidates every markdown module on an edit under the fixture tree', () => {
    const { fire, nodes } = harness()
    fire(`${ROOT}/align_equals/basic_run/input.py`)
    expect(nodes.map(node => node.invalidated)).toStrictEqual([true, undefined])
  })

  it('marks the site config dirty so the markdown cache rebuilds', () => {
    const { fire, site } = harness()
    fire(`${ROOT}/align_equals/basic_run/meta.toml`)
    expect(site.__dirty).toBe(true)
  })

  it('reloads the page on an edit under the fixture tree', () => {
    const { fire, sent } = harness()
    fire(`${ROOT}/align_equals/basic_run/input.py.snap`)
    expect(sent).toStrictEqual([{ type: 'full-reload' }])
  })

  it('leaves an edit outside the fixture tree alone', () => {
    const { fire, nodes, sent, site } = harness()
    fire('/repo/site/rules/alignment/align-equals.md')
    expect(nodes.every(node => node.invalidated === undefined)).toBe(true)
    expect(site.__dirty).toBe(false)
    expect(sent).toStrictEqual([])
  })

  it('reloads without a resolved site config', () => {
    const { fire, nodes, sent } = harness(false)
    expect(() => fire(`${ROOT}/align_equals/basic_run/input.py`)).not.toThrow()
    expect(nodes[0].invalidated).toBe(true)
    expect(sent).toStrictEqual([{ type: 'full-reload' }])
  })

  it('takes a config carrying no vitepress field', () => {
    const plugin = fixtureReloadPlugin(ROOT) as Plugin & { configResolved: (c: unknown) => void }
    expect(() => plugin.configResolved({})).not.toThrow()
  })
})
