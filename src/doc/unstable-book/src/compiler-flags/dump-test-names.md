# `dump-test-names`

The tracking issue for this feature is: [#50297](https://github.com/rust-lang/rust/issues/50297).

------------------------

Listing the tests in a crate normally requires building and linking the test
binary and then running it with `--list`. That makes test discovery cost a full
codegen and link of the crate and of every dependency it links against.

`-Zdump-test-names` reports the tests the `#[test]` harness collected, as JSON,
and then stops the compilation before analysis, codegen and linking. Discovery
therefore needs only the metadata of the crate's dependencies (plus real builds
of any proc-macro dependencies, which have to run during expansion).

```console
$ rustc --test -Zdump-test-names src/lib.rs
```

Writes to standard output by default. `-Zdump-test-names=<path>` writes to
`<path>` instead, and leaves standard output empty.

## Output

```json
{
  "format_version": 1,
  "crate_name": "mycrate",
  "tests": [
    {
      "name": "nested::deeper::deep",
      "kind": "test",
      "ignore": false,
      "ignore_message": null,
      "should_panic": "no",
      "should_panic_message": null,
      "test_type": "unit",
      "source_file": "src/lib.rs",
      "start_line": 42,
      "start_col": 8,
      "end_line": 42,
      "end_col": 12
    }
  ]
}
```

`tests` holds exactly the entries the generated harness registers, in the order
the harness registers them, which is also the order libtest reports them in.
`kind` is `"test"` or `"benchmark"`, so `"{name}: {kind}"` reproduces a line of
`<test binary> --list --format terse`.

The remaining fields mirror the corresponding `test::TestDesc` fields:

* `ignore` and `ignore_message` come from `#[ignore]` / `#[ignore = "..."]`.
* `should_panic` is `"no"` or `"yes"`, and `should_panic_message` carries the
  `expected = "..."` string of `#[should_panic(expected = "...")]`, if any.
* `test_type` is `"unit"`, `"integration"` or `"unknown"`, matching
  `test::TestType`.
* `source_file` and the line/column pair span the test function's name, using
  the same remapping as the emitted `TestDesc`.

## Scope

* The flag is a no-op for a crate that is not compiled with `--test`. That
  includes dependencies and `harness = false` targets, so the flag is safe to
  set for a whole build, for example through `RUSTFLAGS`.
* No output artifact is produced for the crate that is dumped, because
  compilation stops before codegen. Dependency info requested with
  `--emit dep-info` is still written.
* Doctests are not covered; they are collected by rustdoc, not by the
  `#[test]` harness.
