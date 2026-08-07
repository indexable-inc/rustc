// Regression test: `-Zrmeta-strip-spans` must strip only the *location* half
// of a span and preserve its `SyntaxContext`.
//
// Identifier spans carry the identifier's hygiene mark. A dependent crate's
// resolver keys module bindings on (name, namespace, macros-2.0-normalized
// syntax context) when it rebuilds an external module's reduced graph, so two
// macro-generated bindings that share a name and differ only by hygiene are
// distinct bindings. Stripping spans to a wholesale `DUMMY_SP` (whose context
// is the root) collapsed such bindings into one key, and consumers ICEd in
// `rustc_resolve::build_reduced_graph` with "an external binding was already
// defined" (seen in the field on macro-heavy crates: codepage reading
// encoding_rs metadata, thiserror consumers).
//
// This test compiles a producer whose module has several same-name children
// distinguished only by (opaque) hygiene, under both stripping modes, and
// checks that a consumer that forces those children to be decoded compiles.
// It then checks that the fix did not undo the flag's purpose on this same
// hygiene-rich crate: under the full stability flag set, a comment-only edit
// that shifts every following line still produces byte-identical metadata.

//@ ignore-cross-compile

use run_make_support::{rfs, rust_lib_name, rustc};

// Two invocations of a macros-2.0 macro give each generated item a fresh
// opaque hygiene mark, so module `m` gets two `probe` children in the value
// namespace and two `Probe` children in the type namespace, each pair
// distinguished only by the syntax context on the ident's span.
const PRODUCER: &str = r#"#![feature(decl_macro)]

pub macro define_probe() {
    pub struct Probe;
    pub fn probe() {}
}

pub mod m {
    crate::define_probe!();
    crate::define_probe!();
}
"#;

// A glob import of `producer::m` forces the consumer's resolver to build the
// reduced graph for the module, decoding every child's ident (this is where
// the ICE fired).
const CONSUMER: &str = r#"#![allow(unused_imports)]
use producer::m::*;
"#;

const STABILITY_FLAGS: &[&str] =
    &["-Zrmeta-content-svh", "-Zrmeta-strip-spans=all", "-Zrmeta-normalize-src-hash"];

fn produce(source: &str, flags: &[&str]) -> Vec<u8> {
    rfs::write("producer.rs", source);
    rustc()
        .input("producer.rs")
        .crate_name("producer")
        .crate_type("lib")
        .edition("2024")
        .emit("metadata,link")
        .args(flags)
        .run();
    rfs::read(rust_lib_name("producer").replace(".rlib", ".rmeta"))
}

fn consume() {
    rfs::write("consumer.rs", CONSUMER);
    rustc()
        .input("consumer.rs")
        .crate_name("consumer")
        .crate_type("lib")
        .edition("2024")
        .extern_("producer", rust_lib_name("producer"))
        .run();
}

fn main() {
    // Consumers must not ICE, under either stripping mode.
    for mode in ["-Zrmeta-strip-spans=all", "-Zrmeta-strip-spans=non-exported"] {
        produce(PRODUCER, &[mode]);
        consume();
    }

    // Preserving hygiene must not reintroduce source-position churn: on this
    // same hygiene-rich crate, a comment-only edit that shifts every
    // following line still converges to byte-identical metadata under the
    // full stability flag set, and the consumer still compiles against it.
    let base = produce(PRODUCER, STABILITY_FLAGS);
    consume();
    let comment_edit = PRODUCER.replacen(
        "#![feature(decl_macro)]\n",
        "#![feature(decl_macro)]\n// a comment that shifts every following line\n",
        1,
    );
    let edited = produce(&comment_edit, STABILITY_FLAGS);
    assert!(
        base == edited,
        "comment-only edit changed the rmeta of the hygiene-rich crate despite stability flags"
    );
    consume();
}
