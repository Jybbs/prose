import fs            from 'node:fs'
import { isBuiltin } from 'node:module'
import path          from 'node:path'

import { hash }  from 'ohash'
import { build } from 'vite'

import { siteDir }             from '../../shared/paths'
import { readPackageVersions } from '../../shared/version'
import type { BrandAssets }    from './assets'
import type { Card }           from './pool'

// The source of every local module a directory's modules reach, keyed by its
// path from that directory, and the pinned version of every package those
// modules import.
export interface RenderGraph {
  modules  : Record<string, string>
  packages : Record<string, string>
}

export function cardKeyer(
  brand   : BrandAssets,
  graph   : RenderGraph,
  version : string
): (card: Card) => string {
  const base = { brand: hash(brand), graph: hash(graph), version }
  return card => hash({ base, card })
}

// Bundles every module in `dir` through Vite, leaving each bare specifier
// external, and reads the bundler's module graph once the build ends, so every
// module the entries load joins the graph, directly or through another module,
// whether or not the output keeps its code.
export async function readRenderGraph(dir: string = import.meta.dirname): Promise<RenderGraph> {
  const root    = fs.realpathSync(dir)
  const entries = fs.readdirSync(root, { withFileTypes: true })
    .filter(entry => entry.isFile() && !entry.name.startsWith('.'))
    .map(entry => path.join(root, entry.name))
  const loaded: string[] = []

  await build({
    configFile : false,
    logLevel   : 'silent',
    plugins    : [{ buildEnd() { loaded.push(...this.getModuleIds()) }, name: 'render-graph' }],
    root       : root,
    build      : {
      ssr             : true,
      write           : false,
      rolldownOptions : {
        external : id => !id.startsWith('.') && !path.isAbsolute(id),
        input    : entries
      }
    }
  })

  const { files = [], specifiers = [] } =
    Object.groupBy(loaded, id => path.isAbsolute(id) ? 'files' : 'specifiers')
  const packages = new Set(specifiers.filter(id => !isBuiltin(id)).map(packageName))

  return {
    packages : readPackageVersions(siteDir(import.meta.url), [...packages]),
    modules  : Object.fromEntries(files.map(file =>
      [path.relative(root, file), fs.readFileSync(file, 'utf8')]))
  }
}

function packageName(specifier: string): string {
  return specifier.split('/', specifier.startsWith('@') ? 2 : 1).join('/')
}
