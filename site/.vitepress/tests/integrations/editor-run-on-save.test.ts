// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import EditorRunOnSave       from '../../theme/components/integrations/EditorRunOnSave.vue'
import { rendersAccessibly } from '../axe'
import { isHidden }          from '../dom'

import type loader from '../../lib/shared/tools.data'

vi.mock('../../lib/shared/tools.data', async importOriginal =>
  ({ data: await (await importOriginal<{ default: typeof loader }>()).default.load([]) }))

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

describe('EditorRunOnSave', () => {
  it('opens on the first editor with only its face visible', () => {
    const w = mount(EditorRunOnSave)
    expect(w.findAll('.editor-card-row')[0].attributes('aria-selected')).toBe('true')
    const faces = w.findAll('.editor-card-face')
    expect(isHidden(faces[0])).toBe(false)
    expect(isHidden(faces[1])).toBe(true)
  })

  it('sets each row\'s mark in a ring holding no link', () => {
    const w = mount(EditorRunOnSave)
    expect(w.findAll('.tool-ring svg')).toHaveLength(2)
    expect(w.find('.tool-ring a').exists()).toBe(false)
  })

  it('draws each row in gray at rest and marks the selected one under aria-selected', () => {
    const row = mount(EditorRunOnSave).get('.editor-card-row')
    expect(row.classes()).toStrictEqual(expect.arrayContaining([
      'border-l-transparent',
      'text-text-2',
      'aria-selected:border-l-link',
      'aria-selected:text-text-1'
    ]))
    expect(row.get('.tool-ring').classes()).toContain('group-aria-selected:text-link')
    expect(row.get('.italic').classes()).toContain('group-aria-selected:font-600')
  })

  it('swaps the visible face when another row takes the pointer', async () => {
    const w = mount(EditorRunOnSave)
    await w.findAll('.editor-card-row')[1].trigger('mouseenter')
    const faces = w.findAll('.editor-card-face')
    expect(isHidden(faces[0])).toBe(true)
    expect(isHidden(faces[1])).toBe(false)
    expect(faces[1].get('.editor-card-face-target').text()).toBe('init.el')
  })

  it('swaps the visible face when another row takes focus', async () => {
    const w = mount(EditorRunOnSave)
    await w.findAll('.editor-card-row')[1].trigger('focus')
    expect(w.findAll('.editor-card-face')[1].get('.editor-card-face-target').text()).toBe('init.el')
  })

  rendersAccessibly(() => mount(EditorRunOnSave).html())
})
