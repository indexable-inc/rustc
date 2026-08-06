// `-Zdump-test-names` reports the `#[test]`/`#[bench]` functions of a `--test` crate as
// JSON and stops before analysis, codegen and linking, so that a build system can discover
// test names without paying for a test binary.
//
// The correctness bar is parity with the linked binary: the reported names and kinds must
// be byte-for-byte what `<bin> --list --format terse` prints, in the same order.
//
// See https://github.com/rust-lang/rust/issues/50297.

//@ ignore-cross-compile
//@ needs-unwind (the test file contains #[should_panic] tests)

use run_make_support::{cmd, path, rfs, rustc, serde_json};

fn main() {
    // What the real, linked test binary reports.
    rustc().arg("--test").input("tests.rs").output("linked").run();
    let listed = cmd("./linked").args(&["--list", "--format", "terse"]).run().stdout_utf8();
    let from_binary: Vec<&str> = listed
        .lines()
        .filter(|line| line.ends_with(": test") || line.ends_with(": benchmark"))
        .collect();
    assert!(!from_binary.is_empty(), "libtest listed no tests:\n{listed}");

    // What `-Zdump-test-names` reports, with no codegen and no link.
    let dumped = rustc()
        .arg("--test")
        .arg("-Zdump-test-names")
        .input("tests.rs")
        .output("not-written")
        .run()
        .stdout_utf8();
    let doc: serde_json::Value = serde_json::from_str(&dumped).unwrap();
    assert_eq!(doc["format_version"], 1);
    assert_eq!(doc["crate_name"], "tests");

    let tests = doc["tests"].as_array().unwrap();
    let from_flag: Vec<String> = tests
        .iter()
        .map(|test| {
            format!("{}: {}", test["name"].as_str().unwrap(), test["kind"].as_str().unwrap())
        })
        .collect();

    assert_eq!(from_binary, from_flag, "-Zdump-test-names disagrees with `--list --format terse`");

    // Nothing was linked, so no output file exists.
    assert!(!path("not-written").exists(), "-Zdump-test-names produced an output artifact");

    // Spot check the metadata that `--list` does not expose.
    let find = |name: &str| {
        tests
            .iter()
            .find(|test| test["name"] == name)
            .unwrap_or_else(|| panic!("no test named `{name}` in the dump"))
    };

    let ignored = find("ignored_with_message");
    assert_eq!(ignored["ignore"], true);
    assert_eq!(ignored["ignore_message"], "not ready yet");

    let ignored = find("ignored_without_message");
    assert_eq!(ignored["ignore"], true);
    assert!(ignored["ignore_message"].is_null());

    let plain = find("top_level");
    assert_eq!(plain["ignore"], false);
    assert_eq!(plain["should_panic"], "no");
    assert!(plain["should_panic_message"].is_null());
    assert_eq!(plain["source_file"], "tests.rs");
    // The location is the test function's identifier, not the `#[test]` attribute.
    assert_eq!(plain["start_line"], 6);

    assert_eq!(find("panics")["should_panic"], "yes");
    assert!(find("panics")["should_panic_message"].is_null());
    assert_eq!(find("panics_with_message")["should_panic_message"], "boom");
    assert_eq!(
        find("nested::deeper::needs_json_escaping")["should_panic_message"],
        "quoted \" backslash \\ newline"
    );
    assert_eq!(find("a_benchmark")["kind"], "benchmark");

    // Writing to a file instead of stdout gives the same document.
    let stdout = rustc()
        .arg("--test")
        .arg("-Zdump-test-names=names.json")
        .input("tests.rs")
        .run()
        .stdout_utf8();
    assert_eq!(stdout, "", "-Zdump-test-names=<path> should not also write to stdout");
    assert_eq!(rfs::read_to_string("names.json"), dumped);

    // The flag is a no-op for crates that are not compiled with `--test`, so it can be put
    // in `RUSTFLAGS` for a whole build without disturbing dependencies or `harness = false`
    // targets.
    let out = rustc()
        .arg("-Zdump-test-names")
        .crate_type("lib")
        .input("tests.rs")
        .output("liblib.rlib")
        .run();
    assert_eq!(out.stdout_utf8(), "", "-Zdump-test-names printed for a non-`--test` crate");
    assert!(path("liblib.rlib").exists(), "-Zdump-test-names skipped a non-`--test` build");
}
