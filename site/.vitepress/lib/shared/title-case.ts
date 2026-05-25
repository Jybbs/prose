const SMALL_WORDS = new Set([
  'a',   'an',  'and', 'as',  'at',
  'but', 'by',  'for', 'in',  'nor',
  'of',  'off', 'on',  'or',  'out',
  'per', 'so',  'the', 'to',  'up',
  'via', 'with', 'yet'
])

export function toTitleCase(slug: string, separator = '_'): string {
  const words = slug.split(separator)
  return words.map((word, i) => {
    const lower = word.toLowerCase()
    if (i !== 0 && i !== words.length - 1 && word.length <= 4 && SMALL_WORDS.has(lower)) return lower
    return word.charAt(0).toUpperCase() + word.slice(1)
  }).join(' ')
}
