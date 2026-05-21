import { apStyleTitleCase } from 'ap-style-title-case'

const STOPWORDS = [
  'a',     'among',  'an',    'and',  'as',
  'at',    'but',    'by',    'for',  'from',
  'in',    'inside', 'into',  'nor',  'of',
  'on',    'onto',   'or',    'over', 'so',
  'the',   'to',     'under', 'upon', 'via',
  'with',  'within', 'yet'
]

export function toTitleCase(slug: string, separator = '_'): string {
  return apStyleTitleCase(slug.split(separator).join(' '), { stopwords: STOPWORDS })
}
