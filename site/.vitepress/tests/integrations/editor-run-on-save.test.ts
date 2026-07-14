// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import EditorRunOnSave      from '../../theme/components/integrations/EditorRunOnSave.vue'
import { expectAccessible } from '../axe'
import { isHidden }         from '../dom'

vi.mock('../../lib/integrations/editor-configs.data', () => ({
  data: [
    {
      caption  : 'settings.json',
      codeHtml : '<pre>"editor.formatOnSave": true</pre>',
      name     : 'VS Code',
      slug     : 'vscode',
      target   : 'settings.json'
    },
    {
      caption  : 'after-save-hook',
      codeHtml : '<pre>(add-hook ...)</pre>',
      name     : 'Emacs',
      slug     : 'emacs',
      target   : 'init.el'
    }
  ]
}))

const mountCard = () => mount(EditorRunOnSave, { global: { stubs: { Tool: true } } })

describe('EditorRunOnSave', () => {
  it('opens on the first editor with only its face visible', () => {
    const w = mountCard()
    expect(w.findAll('.editor-card-row')[0].attributes('aria-selected')).toBe('true')
    const faces = w.findAll('.editor-card-face')
    expect(isHidden(faces[0])).toBe(false)
    expect(isHidden(faces[1])).toBe(true)
  })

  it('swaps the visible face when another row takes the pointer', async () => {
    const w = mountCard()
    await w.findAll('.editor-card-row')[1].trigger('mouseenter')
    const faces = w.findAll('.editor-card-face')
    expect(isHidden(faces[0])).toBe(true)
    expect(isHidden(faces[1])).toBe(false)
    expect(faces[1].get('.editor-card-face-target').text()).toBe('init.el')
  })

  it('swaps the visible face when another row takes focus', async () => {
    const w = mountCard()
    await w.findAll('.editor-card-row')[1].trigger('focus')
    expect(w.findAll('.editor-card-face')[1].get('.editor-card-face-target').text()).toBe('init.el')
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountCard().html())
  })
})
