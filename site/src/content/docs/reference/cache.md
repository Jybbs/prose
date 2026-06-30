---
title: Cache
---

*Prose* caches per-file pipeline output keyed on the source bytes, the configuration governing the file, the rules the run selects, and the *Prose* version. A repeat `prose check` or `prose format` against an unchanged file collapses to a stat, a hash, and a deserialize, since the cache hit re-emits diagnostics from the cached entry without entering the pipeline.
