import { parseSVGContent, type ParsedSVGContent } from '@iconify/utils'

export function parseSvg(svg: string, label: string): ParsedSVGContent {
  const parsed = parseSVGContent(svg)
  if (!parsed?.attribs.viewBox) throw new Error(`${label} carries no viewBox`)
  return parsed
}
