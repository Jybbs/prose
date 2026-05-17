<template>
  <details class="rule-motivation">
    <summary class="rule-motivation-summary">
      <span class="rule-motivation-caret" aria-hidden="true">›</span>
      <span class="rule-motivation-title">Why this rule?</span>
    </summary>
    <div class="rule-motivation-body">
      <div class="rule-motivation-panel">
        <p class="rule-motivation-kicker">The Problem</p>
        <div class="rule-motivation-content">
          <slot name="problem" />
        </div>
      </div>
      <div class="rule-motivation-panel rule-motivation-panel-fix">
        <p class="rule-motivation-kicker">The Fix</p>
        <div class="rule-motivation-content">
          <slot name="fix" />
        </div>
      </div>
    </div>
  </details>
</template>
