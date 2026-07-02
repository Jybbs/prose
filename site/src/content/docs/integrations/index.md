---
title: Integrations
sidebar:
  label: Overview
---

Every integration on the pages below is a thin wrapper around `prose format` or `prose check`, wired into a different boundary in the development loop. The editor wraps the save event, the pre-commit hook wraps the staging boundary, the CI workflow wraps the merge gate. Each layer runs the same CLI against the same [`[tool.prose]`](/reference/configuration) table and surfaces the same exit codes, so adopting a second integration is configuration rather than a new mental model.
