// Reads the `viewBox` attribute off a raw SVG document. `label` names the
// asset in the error.
export function svgViewBox(svg: string, label: string): string {
  const box = /<svg[^>]*viewBox="([^"]+)"/.exec(svg)?.[1]
  if (box === undefined) throw new Error(`${label} carries no viewBox`)
  return box
}
