import axe       from 'axe-core'
import { JSDOM } from 'jsdom'

export async function expectAccessible(
  html   : string,
  ignore : readonly string[] = []
): Promise<void> {
  const dom = new JSDOM(`<!DOCTYPE html><body>${html}</body>`, { runScripts: 'outside-only' })
  try {
    dom.window.eval(axe.source)
    const realm = dom.window as unknown as { axe: typeof axe }
    const run = await realm.axe.run(dom.window.document.body, {
      rules: {
        'color-contrast' : { enabled: false },
        region           : { enabled: false }
      }
    })
    expect(run.violations.filter(v => !ignore.includes(v.id))).toEqual([])
  }
  finally {
    dom.window.close()
  }
}
