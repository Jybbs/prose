<script setup lang="ts">
import { computed } from 'vue'

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
  <VDropdown theme="run-summary-select" class="inline-block">
    <button
      type="button"
      class="run-summary-select-trigger inline-flex items-baseline gap-0-2em border-none px-2 font-mono text-ube-deep hairline-ube-38 transition-color-and-box-shadow hover:text-ube hover:hairline-ube focus-visible:outline-1 focus-visible:outline-ube"
      :aria-label="label"
      aria-haspopup="listbox"
    >
      <span>{{ selected.mono }}</span>
      <span class="text-0-7em text-ube-mid" aria-hidden="true">▾</span>
    </button>
    <template #popper>
      <ul class="run-summary-opts min-w-max" role="listbox" :aria-label="label">
        <li
          v-for="o in options"
          :key="o.id"
          role="presentation"
        >
          <button
            v-close-popper
            type="button"
            role="option"
            class="run-summary-opt group flex w-full flex-col gap-3 rounded-sm border-none px-11 py-7 text-start transition-colors not-aria-selected:hover:bg-bg-soft aria-selected:bg-ube/12"
            :aria-selected="o.id === model"
            @click="model = o.id"
          >
            <span class="run-summary-opt-mono font-mono text-sm text-text-1 group-aria-selected:text-ube-deep">{{ o.mono }}</span>
            <span v-if="o.preview" class="inline-flex items-baseline gap-0-5em font-mono text-sm">
              <span
                v-if="o.preview.anchor"
                class="flex-none"
                aria-hidden="true"
              >{{ o.preview.anchor }}</span>
              <span class="text-text-2 data-[tint=apricot]:text-apricot data-[tint=celadon]:text-celadon" :data-tint="o.preview.countTint || undefined">
                {{ o.preview.text }}
              </span>
            </span>
          </button>
        </li>
      </ul>
    </template>
  </VDropdown>
</template>
