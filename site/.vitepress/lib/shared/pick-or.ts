// Reads `key` off `record`, returning `fallback` where the record holds no entry.
export function pickOr<T, F>(record: Readonly<Record<string, T>>, key: string, fallback: F): T | F {
  return record[key] ?? fallback
}
