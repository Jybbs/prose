// Serializes a value for a `<script type="application/json">` island payload,
// escaping `<` so no token sequence can close the carrying script element.
export function embedJson(value: unknown): string {
  return JSON.stringify(value).replaceAll('<', '\\u003c')
}
