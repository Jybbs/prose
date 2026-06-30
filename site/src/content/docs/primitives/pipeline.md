---
title: Pipeline
consumedBy:
  - cli
consumes:
  - edit
  - rule-id
  - source
  - suppression-map
layer: orchestration
stability: public
summary: Runs registered rules in deterministic order, reparses between rules, returns the final source.
tagline: deterministic rule runner
---

Runs registered rules in deterministic order, reparses between rules, returns the final source.
