import { directiveParts } from '../../lib/suppression/directive-parts'

describe('directiveParts', () => {
  it('tokenizes a bare form into comment, namespace, and action', () => {
    expect(directiveParts('# prose: off')).toEqual([
      { role : 'comment',   text : '#'      },
      { role : 'namespace', text : 'prose:' },
      { role : 'action',    text : 'off'    }
    ])
  })

  it('carries a bracket payload as a fourth part', () => {
    expect(directiveParts('# prose: skip[<rule>, ...]')).toEqual([
      { role : 'comment',   text : '#'             },
      { role : 'namespace', text : 'prose:'        },
      { role : 'action',    text : 'skip'          },
      { role : 'payload',   text : '[<rule>, ...]' }
    ])
  })

  it.each([
    'fmt: off', '# prose off', '# prose:', '# prose: skip []', '# prose: skip[]'
  ])('throws on the malformed form %j', form => {
    expect(() => directiveParts(form)).toThrow(/does not tokenize/)
  })
})
