export const isLandingId = (id: string): boolean => id === 'index'

export function ogImagePath(id: string): string {
  return isLandingId(id) ? 'og.png' : `og/${id}.png`
}
