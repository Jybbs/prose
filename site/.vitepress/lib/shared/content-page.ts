export const isContentPage = (name: string): boolean =>
  name.endsWith('.md') && name !== 'index.md'
