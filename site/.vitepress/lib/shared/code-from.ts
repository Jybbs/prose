// Recovers the source from a prebuilt highlight, reading only `<pre><code>` so
// the lang chip and copy button stay out of the text.
export function codeFrom(html: string): string {
  return new DOMParser().parseFromString(html, 'text/html')
    .querySelector('pre code')?.textContent?.trimEnd() ?? ''
}
