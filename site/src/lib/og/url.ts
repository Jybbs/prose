// The card URL space: the landing card at `og.png`, every other page's card
// under `og/` off its docs-collection id.

export const isLandingId = (id: string): boolean => id === 'index'

export function ogImagePath(id: string): string {
  return isLandingId(id) ? 'og.png' : `og/${id}.png`
}
