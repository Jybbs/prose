---
title: Ordering Rules
---

The ordering rules reorder sibling AST nodes by a deterministic key while preserving attached comments and inter-section gaps. The shared machinery lives in the orderer primitive, with each rule supplying the classifier closure that names the sort key. A pinning shape lets specific items *(class docstrings, module-level imports above a divider comment)* stay in their authored slot while the rest of the siblings redistribute.
