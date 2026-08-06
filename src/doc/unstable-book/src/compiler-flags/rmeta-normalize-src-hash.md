# `rmeta-normalize-src-hash`

---

This flag replaces the per-file source content hashes recorded in crate
metadata with zeroes. The recorded hash covers a source file's entire raw
text, so by default any edit to a file referenced from metadata, even a
comment edit that moves no token, changes the crate's metadata bytes.

Costs, per consumer of the recorded hash:

* Cross-crate diagnostic snippets: a dependent crate verifies this crate's
  on-disk source against the recorded hash before quoting it in diagnostics.
  The zero hash never matches, so such diagnostics degrade to plain
  `file:line:col` references without a quoted snippet (rather than risking
  quoting stale source).
* Debuginfo: file checksums recorded for this crate's files in dependent
  crates' debug info become zero, so debuggers cannot detect source
  staleness for them.

On its own, this flag stabilizes the metadata against comment edits that
preserve byte length and line positions. For general comment, whitespace,
and private function body edits, combine it with `-Zrmeta-strip-spans=all`
and `-Zrmeta-content-svh`.

The flag has no effect on proc-macro crates.
