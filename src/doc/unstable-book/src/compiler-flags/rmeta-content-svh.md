# `rmeta-content-svh`

---

This flag derives the strict version hash (SVH) stored in crate metadata from
the encoded metadata bytes themselves, instead of from the crate's HIR.

The default HIR-based SVH covers essentially the whole crate, including every
function body and (indirectly) source positions, so any edit to the crate
changes the SVH and with it the metadata, even when nothing a dependent crate
consumes has changed. With this flag, the SVH is a hash of everything else in
the metadata file (plus the session's dependency-tracking hash and the stable
crate id), so it is stable exactly when the rest of the metadata is. Combined
with `-Zrmeta-strip-spans` and `-Zrmeta-normalize-src-hash`, this makes the
`.rmeta` byte-identical across rebuilds after non-interface edits, which lets
content-addressed build systems skip rebuilding dependents.

The link-time consistency check is unaffected: the `.rmeta` and `.rlib`
produced by one compiler invocation embed the same metadata and therefore the
same SVH, and the crate loader continues to compare SVHs for equality.

This flag is rejected in combination with `-Cincremental`, and it does not
change the `crate_hash` query used within the compiling session.
