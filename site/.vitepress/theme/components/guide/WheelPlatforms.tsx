import { defineComponent } from 'vue'

import { NibBadge, ScrollRegion } from '../../../kit'

import { data }           from '../../../lib/guide/platforms.data'
import { platformEmblem } from '../../../lib/shared/emblems'

export const WheelPlatforms = defineComponent({
  name: 'WheelPlatforms',
  setup() {
    return () => (
      <ScrollRegion as="table">
        {{
          default: () => [
            <thead>
                <tr>
                  <th>Triple</th>
                  <th>Platform</th>
                </tr>
              </thead>,
            <tbody>
                {data.map(platform => (
                  <tr key={platform.target}>
                    <td>
                      <code>{platform.target}</code>
                    </td>
                    <td>
                      <span class="nib-chip">
                        <NibBadge emblem={platformEmblem(platform.name)} size={14} />
                      </span>
                      {' '}
                      {platform.label}
                      {platform.manylinux ? <em> (glibc, manylinux)</em> : null}
                    </td>
                  </tr>
                ))}
              </tbody>
          ]
        }}
      </ScrollRegion>
    )
  }
})
