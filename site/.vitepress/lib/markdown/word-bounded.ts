// The lookarounds keep hyphenated and snake_case compounds literal.
export function wordBounded(source: string): RegExp {
  return new RegExp(String.raw`(?<![\w-])(${source})(?![\w-])`, 'g')
}
