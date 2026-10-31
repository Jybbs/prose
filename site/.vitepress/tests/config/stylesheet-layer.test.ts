import fs   from 'node:fs'
import path from 'node:path'

import { layerIconStylesheet, stylesheetLayerPlugin } from '../../lib/config/stylesheet-layer'
import { supportTest }                               from '../support'

const ENTRY = '/site/.vitepress/theme/styles/tailwind.css'

const transform = (code: string, id: string) =>
  (stylesheetLayerPlugin(ENTRY).transform as (code: string, id: string) => unknown)(code, id)

describe('stylesheetLayerPlugin', () => {
  it.each([
    '/site/.vitepress/theme/styles/globals.css',
    '/site/node_modules/vitepress/dist/client/theme-default/styles/vars.css',
    '/site/node_modules/vitepress/dist/client/theme-default/components/VPNav.vue?vue&type=style&index=0&scoped=1&lang.css',
    '\0virtual:prose-palette.css'
  ])('wraps %s in the components layer', id => {
    expect(transform('.a { color: red; }', id)).toStrictEqual({
      code : '@layer components {\n.a { color: red; }\n}',
      map  : null
    })
  })

  it.each([
    ENTRY,
    `${ENTRY}?direct`,
    '/site/.vitepress/theme/styles/globals.css?inline',
    '/site/.vitepress/theme/styles/globals.css?raw',
    '/site/.vitepress/theme/styles/globals.css?url',
    '/site/.vitepress/theme/index.ts'
  ])('leaves %s as it is', id => {
    expect(transform('.a { color: red; }', id)).toBeUndefined()
  })
})

describe('layerIconStylesheet', () => {
  supportTest('wraps the icon stylesheet and leaves every other asset alone', ({ tmpDir }) => {
    fs.writeFileSync(path.join(tmpDir, 'vp-icons.1a2b3c4d.css'), '.vpi-sun{--icon:none}')
    fs.writeFileSync(path.join(tmpDir, 'style.1a2b3c4d.css'), '.a{color:red}')
    layerIconStylesheet(tmpDir)
    expect(fs.readFileSync(path.join(tmpDir, 'vp-icons.1a2b3c4d.css'), 'utf8'))
      .toBe('@layer components {\n.vpi-sun{--icon:none}\n}')
    expect(fs.readFileSync(path.join(tmpDir, 'style.1a2b3c4d.css'), 'utf8')).toBe('.a{color:red}')
  })
})
