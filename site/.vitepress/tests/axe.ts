import axe           from 'axe-core'
import { JSDOM }     from 'jsdom'
import type { Page } from 'playwright'

export interface Finding {
  id      : string
  targets : string[]
}

export const AXE_TIMEOUT_MS = 15_000

// Runs `axe-core` on the page, limited to the rules `runOnly` names where it names any.
export async function audit(page: Page, runOnly?: string[]): Promise<Finding[]> {
  await page.addScriptTag({ content: axe.source })
  return page.evaluate(async options => {
    const { violations } = await (window as unknown as { axe: typeof axe }).axe.run(document, options)
    return violations.map(({ id, nodes }) => ({ id, targets: nodes.map(node => node.target.join(' ')) }))
  }, runOnly ? { runOnly } : {})
}

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
