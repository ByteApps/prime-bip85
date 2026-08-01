//! `bip85-core` is deterministic BY DESIGN: every BIP-85 child comes from
//! the device master seed via `GetSeed`, with no local RNG in the key
//! path (see `tests/rng_backend.rs` for that half of the randomness
//! audit). `spec_vectors.rs` already pins the concrete official BIP-85
//! byte values; this file pins the *property* instead — same input
//! always reproduces, and index/application genuinely change the output
//! — the same "determinism is intentional and scoped" statement the
//! other workspace apps' KDF suites make about their own derivations.

use std::collections::HashSet;

use bip85_core::bip85::{derive, Application};
use bip85_core::{Network, Xprv};

const MASTER: &str = "xprv9s21ZrQH143K2LBWUUQRFXhucrQqBpKdRRxNVq2zBqsx8HVqFk2uYo8kmbaLLHRdqtQpUm98uKfu3vca1LqdGhUtyoFnCNkfmXRyPXLjbKb";

fn root() -> Xprv {
    Xprv::parse(MASTER).expect("spec master xprv parses")
}

#[test]
fn same_root_index_and_application_is_byte_stable() {
    let a = derive(&root(), Application::Bip39 { words: 12 }, 7, Network::Mainnet).unwrap();
    let b = derive(&root(), Application::Bip39 { words: 12 }, 7, Network::Mainnet).unwrap();
    assert_eq!(a.entropy, b.entropy);
    assert_eq!(a.display, b.display);
    assert_eq!(a.path, b.path);
}

#[test]
fn different_indexes_give_different_entropy() {
    let mut seen = HashSet::new();
    for index in [0, 1, 2, 9999] {
        let d = derive(&root(), Application::Bip39 { words: 12 }, index, Network::Mainnet).unwrap();
        assert!(seen.insert(d.entropy.clone()), "index {index} collided with an earlier index");
    }
}

#[test]
fn different_applications_give_different_entropy_at_the_same_index() {
    let mut seen = HashSet::new();
    for app in [
        Application::Bip39 { words: 12 },
        Application::Bip39 { words: 18 },
        Application::Bip39 { words: 24 },
        Application::Wif,
        Application::Xprv,
        Application::Hex { num_bytes: 32 },
        Application::Hex { num_bytes: 64 },
        Application::Pwd { len: 21 },
    ] {
        let d = derive(&root(), app, 0, Network::Mainnet).unwrap();
        assert!(seen.insert(d.entropy.clone()), "{app:?} collided with an earlier application at index 0");
    }
}
