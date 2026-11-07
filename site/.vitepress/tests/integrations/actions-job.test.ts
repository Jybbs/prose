import * as job from '../../lib/integrations/actions-job'
import { HELP }  from '../schema'

const choice = (patch: Partial<job.JobChoice> = {}): job.JobChoice =>
  ({ cache: false, format: job.DEFAULT_FORMAT, ruff: false, ...patch })

describe('checkCommand', () => {
  it('reads the default and the uploaded format off the formats `prose check --help` prints', () => {
    const option = HELP.check.options.find(entry => entry.name === '--output-format')
    expect(option?.default).toBe(job.DEFAULT_FORMAT)
    expect(option?.values).toContain(job.UPLOADED_FORMAT)
  })

  it('leaves the format flag out for the default format', () => {
    expect(job.checkCommand(job.DEFAULT_FORMAT)).toBe('prose check .')
  })

  it('names any other format and writes the uploaded one to a file', () => {
    expect(job.checkCommand('github')).toBe('prose check --output-format github .')
    expect(job.checkCommand(job.UPLOADED_FORMAT)).toBe('prose check --output-format sarif . > prose.sarif')
  })
})

describe('everyChoice', () => {
  it('reaches every format with each step added or not, under distinct keys', () => {
    const choices = job.everyChoice(['text', 'sarif'])
    expect(choices).toHaveLength(8)
    expect(new Set(choices.map(job.choiceKey)).size).toBe(8)
  })
})

describe('jobYaml', () => {
  it('writes the skeleton alone for the default choice', () => {
    const yaml = job.jobYaml(choice())
    expect(yaml).toContain('      - run: prose check .')
    expect(yaml).not.toMatch(/actions\/cache|ruff|permissions|upload-sarif/)
  })

  it('restores the cache ahead of the run', () => {
    const yaml = job.jobYaml(choice({ cache: true }))
    expect(yaml.indexOf('actions/cache@v4')).toBeLessThan(yaml.indexOf('prose check'))
  })

  it('installs and runs Ruff ahead of the prose check', () => {
    const lines = job.jobYaml(choice({ ruff: true })).split('\n')
    expect(lines.indexOf('      - run: ruff check .')).toBeLessThan(lines.indexOf('      - run: prose check .'))
    expect(lines).toContain('      - run: uv tool install ruff')
  })

  it('grants the upload its permission and uploads the file sarif writes', () => {
    const yaml = job.jobYaml(choice({ format: job.UPLOADED_FORMAT }))
    expect(yaml).toContain('security-events: write')
    expect(yaml).toContain('sarif_file: prose.sarif')
  })
})
