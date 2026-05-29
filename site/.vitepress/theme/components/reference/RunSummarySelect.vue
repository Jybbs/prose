<script setup lang="ts">
import { computed } from 'vue'

import type { SelectOption } from './run-summary'

const props = defineProps<{
  ariaLabel : string
  options   : readonly SelectOption[]
}>()

const model = defineModel<string>({ required: true })

const selected = computed(() =>
  props.options.find(o => o.id === model.value) ?? props.options[0]
)
</script>

<template>
  <VDropdown theme="run-summary-select" class="run-summary-select">
    <button
      type="button"
      class="run-summary-select-trigger"
      :aria-label="ariaLabel"
      aria-haspopup="listbox"
    >
      <span>{{ selected.mono }}</span>
      <span class="run-summary-select-caret" aria-hidden="true">▾</span>
    </button>
    <template #popper>
      <ul class="run-summary-opts" role="listbox" :aria-label="ariaLabel">
        <li
          v-for="o in options"
          :key="o.id"
          role="option"
          :aria-selected="o.id === model"
        >
          <button
            v-close-popper
            type="button"
            class="run-summary-opt"
            :class="{ 'is-active': o.id === model }"
            @click="model = o.id"
          >
            <span class="run-summary-opt-mono">{{ o.mono }}</span>
            <span v-if="o.preview" class="run-summary-opt-eg">
              <span
                v-if="o.preview.anchor"
                class="run-summary-opt-eg-anchor"
                aria-hidden="true"
              >{{ o.preview.anchor }}</span>
              <span class="run-summary-opt-eg-text" :data-tint="o.preview.countTint || undefined">
                {{ o.preview.text }}
              </span>
            </span>
          </button>
        </li>
      </ul>
    </template>
  </VDropdown>
</template>
