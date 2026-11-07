import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import type { ConfigRow } from '../../../lib/shared/config-row'
import { InlineProse }    from './InlineProse'

export const ConfigRowTable = defineComponent({
  name  : 'ConfigRowTable',
  props : {
    rows : { required: true, type: Array as PropType<readonly ConfigRow[]> }
  },
  setup(props) {
    return () => (
      <table>
        <thead>
          <tr>
            <th>Key</th>
            <th>Type</th>
            <th>Default</th>
            <th>Meaning</th>
          </tr>
        </thead>
        <tbody>
          {props.rows.map(row => (
            <tr key={row.key}>
              <td>
                <code>{row.key}</code>
              </td>
              <td>
                <InlineProse nodes={row.typeNodes} />
              </td>
              <td>
                {row.default !== 'unset'
                  ? <code>{row.default}</code>
                  : <span>unset</span>}
              </td>
              <td>
                <InlineProse nodes={row.meaningNodes} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    )
  }
})
