<script setup lang="ts">
import { useRoute } from 'vitepress'
import { computed } from 'vue'

import { useAriaHidden }    from '../../../lib/composables/use-aria-hidden'
import { data as glossary } from '../../../lib/glossary/glossary.data'
import { lookup }           from '../../../lib/shared/lookup'
import { stripSuffix }      from '../../../lib/shared/strip-suffix'

const LINK_CLASS = '-mx-16 mt-12 -mb-12 block rounded-b-base-inner-2 bg-popper-border px-16 py-6 '
                 + 'text-center font-mono text-label tracking-meta text-text-1 uppercase no-underline '
                 + 'transition-colors hover:bg-popper-accent-75-divider'

const props = defineProps<{ slug: string }>()

const ariaHidden = useAriaHidden()
const entry      = lookup(glossary.entries, props.slug, 'Glossary entry')
const route      = useRoute()

const tooltipHtml = computed(() => {
  const parts = [
    `<div class="m-0 font-display text-md font-700 whitespace-nowrap text-celadon">${entry.slug}</div>`,
    `<div class="my-6 h-0 border-t-1 border-t-divider" aria-hidden="true"></div>`,
    `<div class="glossary-tooltip-body text-text-2 italic">${entry.definitionHtml}</div>`
  ]
  if (showLink.value) parts.push(`<a href="${entry.href}" class="${LINK_CLASS}">Read more →</a>`)
  return parts.join('')
})

const showLink = computed(() => {
  if (!entry.href) return false
  if (entry.href.includes('#')) return true
  const current = stripSuffix(stripSuffix(route.path, '.html'), '/')
  return current !== stripSuffix(entry.href, '/')
})
</script>

<template>
  <span
    v-tooltip="{
      content            : tooltipHtml,
      html               : true,
      noAutoFocus        : true,
      popperHideTriggers : ['hover'],
      popperTriggers     : ['hover'],
      theme              : 'glossary'
    }"
    class="glossary-anchor underline-draw"
    :tabindex="ariaHidden ? -1 : 0"
  ><slot /></span>
</template>
