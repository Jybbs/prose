<script setup lang="ts">
import Disclosure from './Disclosure.vue'
import Kicker     from './Kicker.vue'
</script>

<template>
  <Disclosure variant="motivation">
    <template #title>Why this rule?</template>
    <div class="rule-motivation-panel">
      <Kicker class="rule-motivation-kicker">The Problem</Kicker>
      <div class="rule-motivation-content">
        <slot name="problem" />
      </div>
    </div>
    <div class="rule-motivation-panel rule-motivation-panel-fix">
      <Kicker class="rule-motivation-kicker">The Fix</Kicker>
      <div class="rule-motivation-content">
        <slot name="fix" />
      </div>
    </div>
  </Disclosure>
</template>
