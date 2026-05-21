export function externalAttrs(href: string | undefined): { rel?: 'noopener'; target?: '_blank' } {
  return isExternal(href) ? { rel: 'noopener', target: '_blank' } : {}
}

export function isExternal(href: string | undefined): boolean {
  return href?.startsWith('http') ?? false
}
