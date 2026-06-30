---
title: GitHub Actions
---

*Prose* compiles cleanly against the standard `ubuntu-latest` runner. The install step fetches the wheel through , the check step runs `prose check`, and the exit code drives the gate. The shapes below trade verbosity for richer surfacing on the PR diff: minimal check, inline workflow-command annotations, and SARIF upload for **Code Scanning**.
