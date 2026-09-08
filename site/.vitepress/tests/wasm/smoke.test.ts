import init, { __wbg_reset_state, format, panic_for_test } from './pkg/prose_wasm.js'

await init()

const SORTED   = 'import a\nimport b\n\nprint(a, b)\n'
const UNSORTED = 'import b\nimport a\n\nprint(a, b)\n'

describe('prose_wasm', () => {
  it('sorts imports through the instantiated module', () => {
    expect(format('', UNSORTED, true).formatted).toBe(SORTED)
  })

  it('returns the findings as structured records', () => {
    expect(format('', 'import os\n\nos.getcwd()\n', true).diagnostics)
      .toContainEqual(expect.objectContaining({ code: 'bare-imports' }))
  })

  it('surfaces an unknown config key as a notice', () => {
    expect(format('no-such-key = 1', 'x = 1\n', true).config_notices)
      .toStrictEqual(['warning: unknown key `no-such-key` in [tool.prose]'])
  })

  it.each([true, false])('marshals the rule-slug vectors with settle %s', settle => {
    const result = format('', 'aa = 1\nb = 2\n', settle)
    expect(result.fired_rules).toContain('align-equals')
    expect(result.unstable_rules).toStrictEqual([])
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

    expect(format('', UNSORTED, true).formatted).toBe(SORTED)
  })
})
