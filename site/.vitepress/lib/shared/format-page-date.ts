export function formatPageDate(ts: number | undefined | null): string {
  return ts ? new Date(ts).toISOString().slice(0, 10) : ''
}
