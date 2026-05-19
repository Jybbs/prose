<script setup lang="ts">
import { computed } from 'vue'

import Chip from '../base/Chip.vue'

import { DOMAIN_META }    from '../../../lib/shared/registries'
import { useCurrentRule } from '../../../lib/shared/route'

const rule = useCurrentRule()
const meta = computed(() => rule.value && DOMAIN_META[rule.value.domain])
const href = computed(() => rule.value && `/rules/#${rule.value.domain}`)
</script>

<template>
  <Chip v-if="rule && meta" variant="domain-chip" :domain="rule.domain" :href="href">
    <span class="domain-chip-badge" aria-hidden="true">{{ meta.badge }}</span>
    <span class="domain-chip-label">{{ meta.label }}</span>
  </Chip>
</template>
