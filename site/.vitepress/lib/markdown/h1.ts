export const markdownH1 = (content: string): string | undefined =>
  content.split('\n').find(line => line.startsWith('# '))?.slice(2).trim()
