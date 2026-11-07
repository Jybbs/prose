import { defineLoader } from 'vitepress'

import * as job                         from './actions-job'
import { getRenderer, renderFencedHtml } from '../markdown/renderer'
import { readHelp }                      from '../reference/cli-help'
import * as paths                        from '../shared/paths'

interface ActionsJob {
  formats   : readonly string[]
  workflows : Record<string, string>
}

declare const data: ActionsJob
export { data }

const root = paths.repoRoot(import.meta.url)

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<ActionsJob> {
    const md     = await getRenderer()
    const values = readHelp(root).check.options
      .find(option => option.name === '--output-format')?.values
    if (!values?.includes(job.DEFAULT_FORMAT)) {
      throw new Error(`\`prose check --help\` lists no \`--output-format ${job.DEFAULT_FORMAT}\``)
    }
    const formats   = [job.DEFAULT_FORMAT, ...values.filter(format => format !== job.DEFAULT_FORMAT)]
    const workflows = await Promise.all(job.everyChoice(formats).map(async choice =>
      [job.choiceKey(choice), await renderFencedHtml(md, job.jobYaml(choice), 'yaml')] as const))
    return { formats, workflows: Object.fromEntries(workflows) }
  }
})
