export const markdownH1 = (content: string): string | undefined =>
  content.match(/^#\s+(.+?)\s*$/m)?.[1]
