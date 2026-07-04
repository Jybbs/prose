import axe    from 'axe-core'
import { JSDOM } from 'jsdom'

export async function axeViolations(html: string): Promise<string[]> {
  const { window } = new JSDOM(`<!doctype html><html lang="en"><body>${html}</body></html>`)
  const globals    = globalThis as Record<string, unknown>
  const saved      = { document: globals.document, window: globals.window }
  Object.assign(globals, { document: window.document, window })
  try {
    const { violations } = await axe.run(window.document.body, { rules: { region: { enabled: false } } })
    return violations.map(violation => violation.id)
  } finally {
    Object.assign(globals, saved)
  }
}
