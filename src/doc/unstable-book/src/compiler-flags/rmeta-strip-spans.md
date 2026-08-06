# `rmeta-strip-spans`

---

This flag replaces spans with dummy spans when encoding crate metadata, so
that the encoded bytes do not depend on source positions. Spans in metadata
are byte offsets into this crate's source files, so by default any edit that
shifts source text (adding a comment, a blank line, editing a function body)
changes the metadata of the crate even when its interface is unchanged.

* `-Zrmeta-strip-spans=none` (default): encode all spans faithfully.
* `-Zrmeta-strip-spans=non-exported`: strip spans except in the metadata that
  dependent crates compile into their own output: encoded MIR bodies (used
  for cross-crate inlining, generic instantiation, and const evaluation) and
  hygiene expansion data. Cost: diagnostics reported in dependent crates can
  no longer point into this crate's source for item-level spans ("function
  defined here" style notes degrade to dummy spans). Debuginfo for
  inlined/generic code is unaffected.
* `-Zrmeta-strip-spans=all`: additionally strip spans inside exported MIR and
  hygiene data, and replace expansion hashes with span-independent ones.
  Additional cost: debuginfo line information in dependent crates for code
  inlined or instantiated from this crate, const-eval error backtraces
  pointing into this crate, and macro-backtrace notes for this crate's
  macros. Concretely, the declaration debuginfo a dependent crate emits for
  a function inlined from this crate carries the dummy span, which resolves
  to the dependent's own first source file at line 1 instead of this
  crate's source; call-site line information in the dependent is
  unaffected. In exchange the metadata does not depend on source positions at
  all: with `-Zrmeta-content-svh` and `-Zrmeta-normalize-src-hash` the
  `.rmeta` is byte-identical after comment, whitespace, and private
  non-inlinable function body edits.

Interface-relevant changes (signatures, visibility, adding or removing items,
and bodies of generic or inlinable functions, whose MIR is exported) still
change the metadata under every mode.

The flag has no effect on proc-macro crates: their metadata consists largely
of span and hygiene data that dependent crates rely on for expansion.
