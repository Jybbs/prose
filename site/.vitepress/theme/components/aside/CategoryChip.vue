<script setup lang="ts">
import './category-chip.css'

import { computed } from 'vue'

import { CATEGORY_META }   from '../../../lib/shared/registries'
import { useCurrentRule }  from '../../../lib/shared/route'

const rule = useCurrentRule()
const meta = computed(() => rule.value && CATEGORY_META[rule.value.category])
</script>

<template>
  <div v-if="rule && meta" :data-category="rule.category" class="category-chip">
    <span class="category-chip-badge" aria-hidden="true">{{ meta.badge }}</span>
    <span class="category-chip-label">{{ meta.label }}</span>
  </div>
</template>
