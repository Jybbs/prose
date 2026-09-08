import type { Plugin } from 'vite'

import { resetFixtureEntries } from './render'

interface VitePressSite {
  __dirty ?: boolean
}

// Keeps the dev server rendering current fixtures. Two caches sit in front of
// `transformPageData`, VitePress holding the rendered markdown and Vite the
// transformed module, so an edit under the fixture tree clears both and reloads
// the page. The watcher takes that tree explicitly because it sits outside the
// Vite root, and the production build never loads this plugin.
export function fixtureReloadPlugin(fixturesRoot: string): Plugin {
  let site: VitePressSite | undefined
  return {
    apply : 'serve',
    name  : 'prose-fixture-reload',
    configResolved(config) {
      site = (config as { vitepress?: VitePressSite }).vitepress
    },
    configureServer(server) {
      server.watcher.add(fixturesRoot)
      server.watcher.on('all', (_event, file) => {
        if (!file.startsWith(fixturesRoot)) return
        resetFixtureEntries()
        // Setting `__dirty` rebuilds the stamp that VitePress mixes into its
        // markdown cache key, so the hook runs again for a source whose text is
        // unchanged.
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
