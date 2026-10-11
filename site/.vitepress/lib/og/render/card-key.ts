import fs            from 'node:fs'
import { isBuiltin } from 'node:module'
import path          from 'node:path'

import { hash }                 from 'ohash'
import { build, type Rolldown } from 'vite'

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
// external, so the bundle lists every local module the entries reach and every
// package they import, directly or through another module.
export async function readRenderGraph(dir: string = import.meta.dirname): Promise<RenderGraph> {
  const root    = fs.realpathSync(dir)
  const entries = fs.readdirSync(root, { withFileTypes: true })
    .filter(entry => entry.isFile() && !entry.name.startsWith('.'))
    .map(entry => path.join(root, entry.name))

  const { output } = await build({
    configFile : false,
    logLevel   : 'silent',
    root       : root,
    build      : {
      ssr             : true,
      write           : false,
      rolldownOptions : {
        external : id => !id.startsWith('.') && !path.isAbsolute(id),
        input    : entries
      }
    }
  }) as Rolldown.RolldownOutput

  const chunks  = output.filter(item => item.type === 'chunk')
  const emitted = new Set(chunks.map(chunk => chunk.fileName))

  const packages = new Set(chunks.flatMap(chunk => [...chunk.imports, ...chunk.dynamicImports])
    .filter(id => !emitted.has(id) && !isBuiltin(id))
    .map(packageName))

  return {
    // Skips each virtual module, whose id Rolldown prefixes with a NUL byte, so
    // only a file on disk joins the graph.
    modules: Object.fromEntries(chunks.flatMap(chunk => chunk.moduleIds)
      .filter(id => !id.startsWith('\0'))
      .map(file => [path.relative(root, file), fs.readFileSync(file, 'utf8')])),
    packages: readPackageVersions(siteDir(import.meta.url), [...packages])
  }
}

function packageName(specifier: string): string {
  return specifier.split('/', specifier.startsWith('@') ? 2 : 1).join('/')
}
