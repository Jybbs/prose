import { format, panic_for_test } from './pkg/prose_wasm.js'

describe('prose_wasm', () => {
  it('sorts imports through the instantiated module', () => {
    expect(format('', 'import b\nimport a\n\nprint(a, b)\n').formatted)
      .toBe('import a\nimport b\n\nprint(a, b)\n')
  })

  it('surfaces the effective config it applied', () => {
    expect(format('code-line-length = 100', 'x = 1\n').config).toContain('code-line-length = 100')
  })

  it('throws when the config is invalid', () => {
    expect(() => format('code-line-length = "wide"', 'x = 1\n')).toThrow(/code-line-length/)
  })

  it('triggers a panic that reaches the console', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => panic_for_test()).toThrow(/unreachable/)
    expect(spy.mock.calls.flat().join(' ')).toContain('smoke-test panic')
    spy.mockRestore()
  })
})
