import { emblemsOf }         from '../../.vitepress/lib/shared/emblems'
import { data as platforms } from '../../.vitepress/lib/usage/platforms.data'

export interface House {
  form     : 'fill' | 'line'
  terminal : 'nib' | 'round'
  tone     : 'duotone' | 'mono'
  weight   : number
}

export const EMBLEMS = emblemsOf(platforms)

export function isClosed(path: string): boolean {
  return /z\s*$/i.test(path)
}
