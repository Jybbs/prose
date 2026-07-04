import path from 'node:path'

import { format } from 'prettier'

const siteRoot = path.resolve(import.meta.dirname, '..', '..')

export const normalize = (text: string): string => text.replaceAll(`${siteRoot}/`, '')

export const formatMarkup = async (text: string): Promise<string> =>
  format(normalize(text), { parser: 'html', printWidth: 100 })
