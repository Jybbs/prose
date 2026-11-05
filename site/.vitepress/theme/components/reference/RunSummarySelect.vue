<script setup lang="ts">
import { computed } from 'vue'

import { Listbox } from '../../../kit'

import type { SelectOption } from '../../../lib/reference/run-summary'

const props = defineProps<{
  label   : string
  options : readonly SelectOption[]
}>()

const model = defineModel<string>({ required: true })

const selected = computed(() =>
  props.options.find(o => o.id === model.value) ?? props.options[0]
)
</script>

<template>
  <Listbox v-model="model" v-slot="listbox" :label="label" :options="options">
    <span class="relative inline-block">
      <button
        v-bind="listbox.trigger"
        type="button"
        class="run-summary-select-trigger inline-flex items-baseline gap-0-2em border-none px-2 font-mono text-link hairline-ube-38 transition-color-and-box-shadow hover:text-text-1 hover:hairline-ube focus-visible:outline-1 focus-visible:outline-ube"
      >
        <span>{{ selected.mono }}</span>
        <span class="text-0-7em text-ube-mid" aria-hidden="true">▾</span>
      </button>
      <div v-bind="listbox.positioner">
        <ul v-bind="listbox.content" class="run-summary-opts z-10000 m-0 min-w-max list-none rounded-sm border-1 border-divider bg-bg p-4 shadow-tooltip">
          <li
            v-for="o in options"
            :key="o.id"
            class="mt-0"
            role="presentation"
          >
            <button
              v-bind="listbox.item(o)"
              type="button"
              class="run-summary-opt group flex w-full flex-col gap-3 rounded-sm border-none px-11 py-7 text-start transition-colors not-aria-selected:hover:bg-bg-soft aria-selected:bg-ube/12"
            >
              <span class="run-summary-opt-mono font-mono text-sm text-text-1 group-aria-selected:text-link">{{ o.mono }}</span>
              <span v-if="o.preview" class="inline-flex items-baseline gap-0-5em font-mono text-sm">
                <span
                  v-if="o.preview.anchor"
                  class="flex-none"
                  aria-hidden="true"
                >{{ o.preview.anchor }}</span>
                <span class="text-text-2 data-[tint=apricot]:text-apricot-code-45 data-[tint=celadon]:text-celadon-code-45" :data-tint="o.preview.countTint || undefined">
                  {{ o.preview.text }}
                </span>
              </span>
            </button>
          </li>
        </ul>
      </div>
    </span>
  </Listbox>
</template>
