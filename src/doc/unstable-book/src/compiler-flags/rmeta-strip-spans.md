# `rmeta-strip-spans`

---

This flag replaces span locations with dummy locations when encoding crate
metadata, so that the encoded bytes do not depend on source positions. Spans
in metadata are byte offsets into this crate's source files, so by default any
edit that shifts source text (adding a comment, a blank line, editing a
function body) changes the metadata of the crate even when its interface is
unchanged.

Only the location half of each span is stripped. The syntax context (hygiene
mark) a span carries is preserved in every mode: identifier spans carry the
identifier's hygiene there, and dependent crates key module bindings on
(name, namespace, normalized syntax context), so macro-generated same-name
bindings that differ only by hygiene must keep distinct contexts for
dependents to compile at all. Syntax contexts are allocated in expansion
order, which depends only on the token stream, not on source positions, so
preserving them does not reintroduce position-dependence.

* `-Zrmeta-strip-spans=none` (default): encode all spans faithfully.
* `-Zrmeta-strip-spans=non-exported`: strip spans except in the metadata that
  dependent crates compile into their own output: encoded MIR bodies (used
  for cross-crate inlining, generic instantiation, and const evaluation),
  hygiene expansion data, and the definition spans of items whose MIR is
  exported. The definition spans must accompany the body spans: a dependent
  crate derives an inlined function's debuginfo declaration file/line from
  the definition span and binds the inlined line rows to that file, so
  stripping it while keeping body spans yields line rows bound to the wrong
  file. Cost: diagnostics reported in dependent crates can no longer point
  into this crate's source for item-level spans of items without exported
  MIR ("function defined here" style notes degrade to dummy spans).
  Stability boundary: because the preserved spans are byte offsets and the
  referenced files' length and line tables must stay real for those spans
  to resolve correctly in dependents, only byte-position-preserving edits
  (same-length comment rewrites, same-length body edits of non-exported
  functions) leave the metadata byte-identical in this mode; any
  length-changing edit, even a comment appended at end of file, perturbs
  the encoded source file record. This mode is for toolchains that want
  the SVH and source-hash normalization plus reduced item-span surface
  with zero debuginfo cost; use `all` when byte-stability under general
  non-interface edits is the goal.
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
