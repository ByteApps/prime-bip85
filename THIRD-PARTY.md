# Third-party libraries

Direct dependencies of this app and its `bip85-core` library. The complete transitive list (with exact versions) is pinned in [`Cargo.lock`](Cargo.lock).

## Rust crates

| Library | Version | License | Used for |
|---|---|---|---|
| [k256](https://crates.io/crates/k256) | 0.13 | Apache-2.0 OR MIT | secp256k1 scalar math for BIP-32 derivation |
| [hmac](https://crates.io/crates/hmac) | 0.12 | MIT OR Apache-2.0 | HMAC-SHA512 (BIP-32/BIP-85 derivation) |
| [sha2](https://crates.io/crates/sha2) | 0.10 | MIT OR Apache-2.0 | SHA-256/512 hashing |
| [ripemd](https://crates.io/crates/ripemd) | 0.1 | MIT OR Apache-2.0 | RIPEMD-160 (BIP-32 fingerprints) |
| [pbkdf2](https://crates.io/crates/pbkdf2) | 0.12 | MIT OR Apache-2.0 | BIP-39 seed stretching |
| [bs58](https://crates.io/crates/bs58) | 0.5 | MIT OR Apache-2.0 | Base58Check (WIF, XPRV) |
| [zeroize](https://crates.io/crates/zeroize) | 1 | Apache-2.0 OR MIT | Wiping secrets from memory |
| [log](https://crates.io/crates/log) | 0.4 | MIT OR Apache-2.0 | Logging facade |
| [hex](https://crates.io/crates/hex) (dev) | 0.4 | MIT OR Apache-2.0 | Test vectors |

## Vendored code

| Component | Origin | Role |
|---|---|---|
| `vendor/getrandom/` | KeyOS source (getrandom 0.2 fork) | Entropy override: hardware TRNG server on KeyOS builds, stock behavior on host |
| `vendor/security-api/` | KeyOS v1.2.1 source, adapted to SDK 0.4.0 conventions | `os/security` API client (`GetSeed`) |

## Foundation SDK / KeyOS platform

Provided by the installed Foundation SDK (path dependencies, not crates.io):

| Component | Role |
|---|---|
| `server` (KeyOS) | App runtime, KeyOS service messaging, filesystem API |
| `xous-api-log` | Log output to the KeyOS log server |
| `slint-keyos-platform` (+ `-build`) | [Slint](https://slint.dev) UI runtime, QR rendering, and build integration for KeyOS |
| `foundation-themes` | Design tokens and light/dark theming |

The Slint UI toolkit itself is licensed under GPL-3.0-only OR the Slint
Royalty-free / commercial licenses. **This app elects the GPL**, which is why
it is GPL-3.0-or-later. That is not a free choice: section 3 of the Slint
Royalty-free license excludes embedded systems, and a Passport Prime is one, so
on-device the GPL is the only option that costs nothing. KeyOS's own API crates
(`server`, `fs`, `crypto`, `security`, ...) are GPL-3.0-or-later as well. Taking
this app closed-source would require a paid Slint license *and* a resolution of
the KeyOS side.
