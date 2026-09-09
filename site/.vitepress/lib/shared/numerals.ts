// Joins a count to its noun, pluralizing with a regular `-s`.
export function counted(n: number, noun: string): string {
  return `${n} ${noun}${n === 1 ? '' : 's'}`
}

export function formatFolio(n: number): string {
  return String(n).padStart(2, '0')
}
