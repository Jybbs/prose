import axe       from 'axe-core'
import { JSDOM } from 'jsdom'

export const AXE_TIMEOUT_MS = 15_000

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
    // Copies the violations out of the JSDOM realm axe runs in, whose array
    // prototype differs from this realm's and fails a strict compare.
    expect([...run.violations].filter(v => !ignore.includes(v.id))).toStrictEqual([])
  }
  finally {
    dom.window.close()
  }
}

export function rendersAccessibly(html: () => string): void {
  it('renders with no axe violations', async () => {
    await expectAccessible(html())
  }, AXE_TIMEOUT_MS)
}
