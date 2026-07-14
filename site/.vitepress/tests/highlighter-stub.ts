// Tokenizes each line as one styled token, so the typewriter suites read
// token content and style off the output without loading shiki. A
// whitespace-only line comes back unstyled, the way shiki leaves a token no
// theme rule matches.
export function codeHighlighter(): Promise<{
  codeToTokens: (text: string) => { tokens: { content: string, htmlStyle?: object }[][] }
}> {
  return Promise.resolve({
    codeToTokens: (text: string) => ({
      tokens: text.split('\n').map(line => {
        if (line === '') return []
        return line.trim() === ''
          ? [{ content: line, htmlStyle: undefined }]
          : [{ content: line, htmlStyle: { color: 'red' } }]
      })
    })
  })
}
