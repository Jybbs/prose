import { __wbg_reset_state, format, panic_for_test } from './pkg/prose_wasm.js'

describe('prose_wasm', () => {
  it('sorts imports through the instantiated module', () => {
    expect(format('', 'import b\nimport a\n\nprint(a, b)\n', true).formatted)
      .toBe('import a\nimport b\n\nprint(a, b)\n')
  })

  it('returns the findings as structured records', () => {
    expect(format('', 'import os\n\nos.getcwd()\n', true).diagnostics)
      .toContainEqual(expect.objectContaining({ code: 'bare-imports' }))
  })

  it('surfaces an unknown config key as a notice', () => {
    expect(format('no-such-key = 1', 'x = 1\n', true).config_notices)
      .toEqual(['warning: unknown key `no-such-key` in [tool.prose]'])
  })

  it('skips the settle walk when the caller asks for no settle check', () => {
    expect(format('', 'alpha = 1\nb = 22\n', false).unstable_rules).toEqual([])
  })

  it('throws when the config is invalid', () => {
    expect(() => format('code-line-length = "wide"', 'x = 1\n', true)).toThrow(/code-line-length/)
  })

  it('recovers from a panic through a reset and formats again', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => panic_for_test()).toThrow(/unreachable/)
    expect(spy.mock.calls.flat().join(' ')).toContain('smoke-test panic')
    spy.mockRestore()

    __wbg_reset_state()

    expect(format('', 'import b\nimport a\n\nprint(a, b)\n', true).formatted)
      .toBe('import a\nimport b\n\nprint(a, b)\n')
  })
})
