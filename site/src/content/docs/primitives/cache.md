---
title: Cache
consumedBy:
  - cli
consumes:
  - source
layer: analysis
stability: internal
summary: User-level on-disk cache keyed on `(source ++ config ++ rules ++ version)`, collapsing repeat runs to a stat plus a hash plus a deserialize.
tagline: content-addressed result cache
---

User-level on-disk cache keyed on `(source ++ config ++ rules ++ version)`, collapsing repeat runs to a stat plus a hash plus a deserialize.
