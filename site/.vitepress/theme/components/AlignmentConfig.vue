<template>
  <table>
    <thead>
      <tr>
        <th>Key</th>
        <th>Type</th>
        <th>Default</th>
        <th>Meaning</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td><code>enabled</code></td>
        <td>bool</td>
        <td><code>true</code></td>
        <td>Toggle the rule on or off</td>
      </tr>
      <tr>
        <td><code>max-shift</code></td>
        <td>positive int</td>
        <td><code>8</code></td>
        <td>Ceiling on per-line padding</td>
      </tr>
      <tr>
        <td><code>max-shift-policy</code></td>
        <td><code>"split"</code> | <code>"drop"</code> | <code>"skip"</code></td>
        <td><code>"split"</code></td>
        <td>How to handle a group whose widest member exceeds <code>max-shift</code>. See <a href="/guide/configuration#per-rule-knobs">the per-rule knobs</a> for the full semantics</td>
      </tr>
    </tbody>
  </table>
</template>
