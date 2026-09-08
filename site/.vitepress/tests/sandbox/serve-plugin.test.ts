import fs   from 'node:fs'
import path from 'node:path'

import { serveWasmPlugin } from '../../lib/sandbox/serve-plugin'
import { supportTest }     from '../support'

type Middleware = (
  request  : { url?: string },
  response : { end: (body: unknown) => void, setHeader: (name: string, value: string) => void },
  next     : () => void
) => void

// Fills the temporary directory with the served tree plus one file outside it,
// which gives the traversal case a real target.
const wasmTest = supportTest.extend<{ publicDir: string }>({
  publicDir: async ({ tmpDir }, use) => {
    fs.mkdirSync(path.join(tmpDir, 'wasm'))
    fs.writeFileSync(path.join(tmpDir, 'wasm', 'prose.wasm'), 'BYTES')
    fs.writeFileSync(path.join(tmpDir, 'wasm', 'prose.js'), 'export {}')
    fs.writeFileSync(path.join(tmpDir, 'wasm', 'prose.map'), '{}')
    fs.writeFileSync(path.join(tmpDir, 'secret.txt'), 'no')
    await use(tmpDir)
  }
})

// The plugin registers its middleware through `server.middlewares.use`, so the
// fake server captures the one function under test.
const middleware = (publicDir: string): Middleware => {
  let captured!: Middleware
  const server = { middlewares: { use: (fn: Middleware) => { captured = fn } } }
  const hook   = serveWasmPlugin(publicDir).configureServer as (s: unknown) => void
  hook(server)
  return captured
}

const request = (publicDir: string, url?: string) => {
  const response = {
    end       : vi.fn<(body: unknown) => void>(),
    setHeader : vi.fn<(name: string, value: string) => void>()
  }
  const next = vi.fn<() => void>()
  middleware(publicDir)({ url }, response, next)
  return { next, response }
}

describe('serveWasmPlugin', () => {
  wasmTest('serves only under the dev server', ({ publicDir }) => {
    expect(serveWasmPlugin(publicDir).apply).toBe('serve')
  })

  wasmTest.for([
    ['prose.wasm', 'application/wasm'],
    ['prose.js',   'text/javascript'],
    ['prose.map',  'application/octet-stream']
  ])('answers %s with %s', ([file, type], { publicDir }) => {
    const { next, response } = request(publicDir, `/wasm/${file}`)
    expect(response.setHeader).toHaveBeenCalledExactlyOnceWith('Content-Type', type)
    expect(response.end).toHaveBeenCalledOnce()
    expect(next).not.toHaveBeenCalled()
  })

  wasmTest('reads the file past the import query the dev server appends', ({ publicDir }) => {
    const { response } = request(publicDir, '/wasm/prose.wasm?import')
    expect(response.end).toHaveBeenCalledOnce()
  })

  wasmTest.for([
    ['a path outside /wasm/',    '/assets/app.js'],
    ['a traversal out of wasm',  '/wasm/../secret.txt'],
    ['a file that is not there', '/wasm/absent.wasm'],
    ['a request carrying no url', undefined]
  ])('passes %s along', ([, url], { publicDir }) => {
    const { next, response } = request(publicDir, url)
    expect(next).toHaveBeenCalledOnce()
    expect(response.end).not.toHaveBeenCalled()
  })
})
