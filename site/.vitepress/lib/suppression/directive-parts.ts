const FORM_SHAPE = /^# ([a-z]+:) ([a-z]+)(\[.+\])?$/

export type PartRole = 'action' | 'comment' | 'namespace' | 'payload'

export interface DirectivePart {
  role : PartRole
  text : string
}

export function directiveParts(form: string): DirectivePart[] {
  const match = FORM_SHAPE.exec(form)
  if (match === null) {
    throw new Error(`directive parts: form "${form}" does not tokenize`)
  }
  const [, namespace, action, payload] = match
  const parts: DirectivePart[] = [
    { role : 'comment',   text : '#'       },
    { role : 'namespace', text : namespace },
    { role : 'action',    text : action    }
  ]
  if (payload !== undefined) parts.push({ role: 'payload', text: payload })
  return parts
}
