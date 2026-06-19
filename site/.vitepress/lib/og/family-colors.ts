import fs   from 'node:fs'
import path from 'node:path'

import { FAMILY_ORDER, type RuleFamily } from '../shared/registries'

export function resolveFamilyColors(srcDir: string): Record<RuleFamily, string> {
  const css   = fs.readFileSync(path.join(srcDir, '.vitepress', 'theme', 'styles', 'tokens.css'), 'utf8')
  const decl  = (name: string) => css.match(new RegExp(`--${name}\\s*:\\s*([^;]+);`))?.[1].trim() ?? ''
  const hexOf = (family: RuleFamily) => {
    const alias = decl(`prose-c-family-${family}`)
    const named = alias.match(/var\(--(.+?)\)/)?.[1]
    return named ? decl(named) : alias
  }
  return Object.fromEntries(
    FAMILY_ORDER.map(family => [family, hexOf(family)])
  ) as Record<RuleFamily, string>
}
