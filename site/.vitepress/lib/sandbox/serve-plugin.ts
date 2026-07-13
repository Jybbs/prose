import fs   from 'node:fs'
import path from 'node:path'

import type { Plugin } from 'vite'

const CONTENT_TYPES: Record<string, string> = {
  '.js'   : 'text/javascript',
  '.wasm' : 'application/wasm'
}

// The dev server tags a dynamic `import()` as an import request and rejects
// import requests for `/public` files, so this middleware answers `/wasm/`
// requests with the raw bytes before that check runs. The production build
// serves the same files statically and never loads this plugin.
export function serveWasmPlugin(publicDir: string): Plugin {
  return {
    apply : 'serve',
    name  : 'prose-serve-wasm',
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const url  = (request.url ?? '').split('?')[0]
        const file = path.join(publicDir, url)
        if (!url.startsWith('/wasm/') || !file.startsWith(path.join(publicDir, 'wasm'))) return next()
        if (!fs.existsSync(file)) return next()
        response.setHeader('Content-Type', CONTENT_TYPES[path.extname(file)] ?? 'application/octet-stream')
        response.end(fs.readFileSync(file))
      })
    }
  }
}
