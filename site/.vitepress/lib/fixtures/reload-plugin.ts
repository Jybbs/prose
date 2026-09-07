import type { Plugin } from 'vite'

import { resetFixtureEntries } from './render'

// Keeps the dev server rendering current fixtures. A page takes its entries
// from `transformPageData`, and two caches sit in front of that hook, VitePress
// holding the rendered markdown and Vite the transformed module. An edit under
// the fixture tree clears both and reloads the page, and that tree sits outside
// the Vite root, leaving the watcher to take it explicitly. The production build
// never loads this plugin.
export function fixtureReloadPlugin(fixturesRoot: string): Plugin {
  let site: { __dirty?: boolean } | undefined
  return {
    apply : 'serve',
    name  : 'prose-fixture-reload',
    configResolved(config) {
      site = (config as { vitepress?: { __dirty?: boolean } }).vitepress
    },
    configureServer(server) {
      server.watcher.add(fixturesRoot)
      server.watcher.on('all', (_event, file) => {
        if (!file.startsWith(fixturesRoot)) return
        resetFixtureEntries()
        // Setting `__dirty` rebuilds the stamp VitePress mixes into its markdown
        // cache key, so the hook runs again for a source whose text is unchanged.
        // oxlint-disable-next-line no-underscore-dangle -- VitePress names this field
        if (site) site.__dirty = true
        const { hot, moduleGraph } = server.environments.client
        for (const node of moduleGraph.idToModuleMap.values()) {
          if (node.file?.endsWith('.md')) moduleGraph.invalidateModule(node)
        }
        hot.send({ type: 'full-reload' })
      })
    }
  }
}
