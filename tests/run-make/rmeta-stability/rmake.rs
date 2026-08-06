// Test that `-Zrmeta-content-svh`, `-Zrmeta-strip-spans=all` and
// `-Zrmeta-normalize-src-hash` together make a library crate's `.rmeta`
// byte-identical across non-interface edits:
//
// 1. an identical rebuild (this must also hold with the flags off),
// 2. a comment-only edit,
// 3. a whitespace edit that shifts every subsequent line,
// 4. a private non-generic non-inlinable function body edit,
//
// while interface-relevant edits must still change the metadata:
//
// 5. a generic function body edit (its MIR is encoded for downstream
//    instantiation),
// 6. a signature change.
//
// `-Zrmeta-normalize-src-hash` alone is also checked to stabilize the
// weakest edit class: a comment edit that preserves byte length and line
// positions, which perturbs nothing but the recorded source file hash.

//@ ignore-cross-compile

use run_make_support::{rfs, rust_lib_name, rustc};

// The private function carries `#[inline(never)]` so that it is not
// auto-selected for cross-crate inlining at -Copt-level=3; a small private
// function without it would have its (legitimately body-dependent) MIR
// encoded into the metadata.
const BASE: &str = r#"//! Demo crate for rmeta stability.

// helper does the arithmetic heavy lifting

pub struct Config {
    pub retries: u32,
}

pub fn make_config() -> Config {
    Config { retries: 3 }
}

pub fn double(x: u64) -> u64 {
    helper(x) * 2
}

#[inline(never)]
fn helper(x: u64) -> u64 {
    x + 1
}

pub fn generic_max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

#[inline]
pub fn inlined_add(a: u32, b: u32) -> u32 {
    a + b
}
"#;

const STABILITY_FLAGS: &[&str] =
    &["-Zrmeta-content-svh", "-Zrmeta-strip-spans=all", "-Zrmeta-normalize-src-hash"];

fn compile(source: &str, flags: &[&str]) -> Vec<u8> {
    rfs::write("lib.rs", source);
    rustc()
        .input("lib.rs")
        .crate_name("demo")
        .crate_type("lib")
        .edition("2024")
        .opt_level("3")
        .emit("metadata,link")
        .args(flags)
        .run();
    rfs::read(rust_lib_name("demo").replace(".rlib", ".rmeta"))
}

fn main() {
    // Edit classes 2 to 4: must be byte-identical with the stability flags.
    let comment_edit = BASE.replace("x + 1", "x + 1 // tweaked comment");
    let whitespace_edit = BASE.replacen(
        "//! Demo crate for rmeta stability.\n",
        "//! Demo crate for rmeta stability.\n\n",
        1,
    );
    let private_body_edit = BASE.replace("x + 1", "x + 2");
    // Controls 5 and 6: must still differ with the stability flags.
    let generic_body_edit = BASE.replace("if a > b", "if a >= b");
    let signature_edit =
        BASE.replace("pub fn double(x: u64)", "pub fn double(x: u64, _unused: u8)");
    // Class 2c: same byte length, same line structure, different comment text.
    let comment_edit_same_len =
        BASE.replace("arithmetic heavy lifting", "arithmetic heavy WORKING");
    assert_eq!(BASE.len(), comment_edit_same_len.len());

    // Class 1: identical rebuilds must be byte-identical even without flags.
    let base_no_flags = compile(BASE, &[]);
    let rebuild_no_flags = compile(BASE, &[]);
    assert!(base_no_flags == rebuild_no_flags, "identical rebuild changed the rmeta (flags off)");

    let base = compile(BASE, STABILITY_FLAGS);
    let rebuild = compile(BASE, STABILITY_FLAGS);
    assert!(base == rebuild, "identical rebuild changed the rmeta (flags on)");

    let comment = compile(&comment_edit, STABILITY_FLAGS);
    assert!(base == comment, "comment-only edit changed the rmeta despite stability flags");

    let whitespace = compile(&whitespace_edit, STABILITY_FLAGS);
    assert!(base == whitespace, "whitespace-only edit changed the rmeta despite stability flags");

    let private_body = compile(&private_body_edit, STABILITY_FLAGS);
    assert!(
        base == private_body,
        "private non-inlinable body edit changed the rmeta despite stability flags"
    );

    // The controls must keep churning: a stability scheme that hides interface
    // changes would let dependents link against incompatible artifacts.
    let generic_body = compile(&generic_body_edit, STABILITY_FLAGS);
    assert!(
        base != generic_body,
        "generic function body edit did NOT change the rmeta; exported MIR must stay visible"
    );

    let signature = compile(&signature_edit, STABILITY_FLAGS);
    assert!(
        base != signature,
        "signature change did NOT change the rmeta; interface changes must stay visible"
    );

    // `-Zrmeta-normalize-src-hash` alone must stabilize a length-and-line
    // preserving comment edit (the only churn there is the source file hash).
    let base_src_hash_only = compile(BASE, &["-Zrmeta-normalize-src-hash"]);
    let comment_2c = compile(&comment_edit_same_len, &["-Zrmeta-normalize-src-hash"]);
    assert!(
        base_src_hash_only == comment_2c,
        "length-preserving comment edit changed the rmeta despite -Zrmeta-normalize-src-hash"
    );
}
