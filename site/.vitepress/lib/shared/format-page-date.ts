export function formatPageDate(ts: number | undefined | null): string {
  return ts ? new Date(ts).toLocaleDateString('en-CA', { timeZone: 'UTC' }) : ''
}
