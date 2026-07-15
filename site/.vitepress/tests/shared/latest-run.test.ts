import { latestRun } from '../../lib/shared/latest-run'

describe('latestRun', () => {
  it('leaves the only run in flight unsuperseded', () => {
    const run = latestRun()
    expect(run.begin()()).toBe(false)
  })

  it('supersedes an earlier run once a newer one begins', () => {
    const run        = latestRun()
    const superseded = run.begin()
    const latest     = run.begin()
    expect(superseded()).toBe(true)
    expect(latest()).toBe(false)
  })

  it('supersedes the run in flight without starting one', () => {
    const run     = latestRun()
    const running = run.begin()
    run.cancel()
    expect(running()).toBe(true)
  })
})
