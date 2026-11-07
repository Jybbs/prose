import { defineComponent } from 'vue'

import { data as sources } from '../lib/kit/catalog.data'

import { declarations } from './catalog'
import { DocHeading }   from './chips/DocHeading'

// Renders every declared piece of the kit live beside its source, in the order
// `declarations` lists them.
export default defineComponent({
  name : 'KitCatalog',
  setup() {
    return () => [...declarations].map(([name, { examples, use }]) => (
      <section key={name}>
        <DocHeading id={name.toLowerCase()} title={name} />
        <p>{use}</p>
        <div class="grid grid-cols-minmax-0-95fr-minmax-1-5fr items-start gap-1-25rem max-tablet:grid-cols-1fr">
          <div class="flex min-w-0 flex-col gap-0-85rem panel bg-bg-soft px-1-25rem py-0-9rem">
            {examples.map(({ label, render }) => (
              <figure class="m-0 flex flex-col gap-0-3rem" key={label}>
                <figcaption class="kicker">{label}</figcaption>
                <div>{render()}</div>
              </figure>
            ))}
          </div>
          <figure class="m-0 min-w-0 overflow-hidden panel bg-bg">
            <figcaption class="border-b-1 border-b-divider px-1-25rem py-0-3rem font-mono text-0-68rem text-text-2">
              {sources[name]!.file}
            </figcaption>
            <div class="flush-code-block" innerHTML={sources[name]!.html} />
          </figure>
        </div>
      </section>
    ))
  }
})
