# Sandbox

The sandbox formats Python the moment you edit it, running the same core *Prose* applies on the command line without anything leaving the page. The formatter compiles to a [WebAssembly](https://webassembly.org) module that loads on demand, so the source settles into shape as you type.

<ProseSandbox />

The editor opens on a case lifted from the fixture corpus, so the first result already shows several rules settle together, wherein the fields sort, the annotation `:` and default `=` columns align, the docstring reshapes, and the blank lines normalize. Replace it with your own source to watch *Prose* rewrite it against the default configuration.

A source that does not parse, or a configuration the core rejects, surfaces its message on the formatted side rather than failing quietly, leaving the editor untouched so you can correct it in place.
