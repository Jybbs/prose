// The design-system tokens as a typed map. `resolveToken` reads a value
// following one `var()` alias, and `tokensToCss` regenerates the `:root`
// custom properties for the browser.

const TOKENS: Record<string, string> = {
  'family-alignment'     : 'var(--prose-palette-eureka)',
  'family-cli'           : 'var(--prose-palette-ube-night)',
  'family-docs'          : 'var(--prose-palette-celadon)',
  'family-engine'        : 'var(--prose-palette-ube)',
  'family-formatting'    : 'var(--prose-palette-heath)',
  'family-layout'        : 'var(--prose-palette-toronto)',
  'family-lint'          : 'var(--prose-palette-apricot)',
  'family-ordering'      : 'var(--prose-palette-chambray)',
  'font-base'            : `'Lora Variable', Georgia, "Times New Roman", serif`,
  'font-display'         : `'Fraunces Variable', Georgia, "Times New Roman", serif`,
  'font-mono'            : `'JetBrains Mono Variable', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace`,
  'palette-apricot'      : '#e8876f',
  'palette-casper'       : '#adbdcd',
  'palette-celadon'      : '#8cc5a3',
  'palette-chambray'     : '#7db3e0',
  'palette-champagne'    : '#f0e9bc',
  'palette-dexter'       : '#6db0b5',
  'palette-eureka'       : '#e8c840',
  'palette-grams-hair'   : '#f6f8fa',
  'palette-heath'        : '#c08597',
  'palette-oat'          : '#cdbda5',
  'palette-rainee'       : '#b8c8a8',
  'palette-toronto'      : '#5069ad',
  'palette-ube'          : '#8a80cb',
  'palette-ube-deep'     : 'color-mix(in oklch, var(--prose-palette-ube), black 22%)',
  'palette-ube-mid'      : 'color-mix(in oklch, var(--prose-palette-ube), white 18%)',
  'palette-ube-night'    : 'color-mix(in oklch, var(--prose-palette-ube), black 45%)',
  'palette-ube-pale'     : 'color-mix(in oklch, var(--prose-palette-ube), white 36%)',
  'palette-whiskey'      : '#d4a574',
  'palette-woodsmoke'    : '#17171b',
  'role-accent'          : 'var(--prose-palette-chambray)',
  'role-error'           : 'var(--prose-palette-apricot)',
  'role-link-hover'      : 'var(--prose-palette-ube-deep)',
  'role-warning'         : 'var(--prose-palette-eureka)',
  'section-integrations' : 'var(--prose-palette-rainee)',
  'section-primitives'   : 'var(--prose-palette-dexter)',
  'section-reference'    : 'var(--prose-palette-casper)',
  'section-usage'        : 'var(--prose-palette-oat)'
}

// Reads a token's value, following one `var()` alias so an aliased token and a
// `color-mix()` blend both resolve to their first referenced token's value,
// leaving the concrete blend to the color consumer.
export function resolveToken(name: string): string {
  const value      = TOKENS[name] ?? ''
  const referenced = value.match(/var\(--prose-([\w-]+)\)/)?.[1]
  return referenced ? (TOKENS[referenced] ?? '') : value
}

export function tokensToCss(): string {
  const lines = Object.entries(TOKENS).map(([name, value]) => `  --prose-${name}: ${value};`)
  return `:root {\n${lines.join('\n')}\n}\n`
}
