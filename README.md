# <img src="resources/icon.svg" alt="" width="42" align="top" /> BIP-85

**Bitcoin · Seeds** — one backup to rule them all: derive every wallet you'll ever hand out from the seed you already protect.

BIP-85 turns your Passport Prime's master seed into a family tree of independent secrets. Need a seed for a hot wallet, a gift, a test device, a friend getting started — or a password that can never be forgotten? Derive a child — a fresh 12-, 18-, or 24-word mnemonic, a WIF key, an XPRV, raw entropy, or a strong password — and hand it out knowing that no child can ever reveal its siblings or the parent. Lose everything, restore your one seed phrase, and every child you ever derived comes back, identical. Fully offline, like everything on Prime.

<p align="center">
  <img src="screenshots/home.png" alt="Home — application picker and index stepper" width="280">
  &nbsp;
  <img src="screenshots/words.png" alt="Derived 12-word mnemonic" width="280">
  &nbsp;
  <img src="screenshots/seedqr.png" alt="SeedQR view" width="280">
</p>

## What it derives

| Application | Output |
|---|---|
| BIP-39 · 12 / 18 / 24 words | a mnemonic for another wallet |
| WIF | a Bitcoin Core `sethdseed` key |
| XPRV | a BIP-32 root for coordinators |
| HEX · 32 / 64 bytes | raw entropy for anything else |
| Password · 20–86 chars | a base64 password, 120–512 bits (default 21 chars ≈ 126 bits) for anything with a login |

## Features

- **Standards-compliant by proof** — pinned to the official [BIP-85](https://github.com/bitcoin/bips/blob/master/bip-0085.mediawiki) test vectors, and every application cross-verified byte-for-byte against an independent BIP-85 implementation: same seed in, same child out, on all eight derivation types.
- **SeedQR out** — show any mnemonic as a SeedQR (SeedSigner standard) for direct camera import into any SeedQR-aware wallet; WIF/XPRV/HEX render as plain QRs.
- **Fingerprints for verification** — the home screen shows your master seed's BIP-32 fingerprint, and every BIP-39/XPRV child shows *its* fingerprint — so you can confirm a restored wallet imported the right child at a glance.
- **10,000 children per application** — indexes 0–9999 via ±1/±100 stepper buttons; no keyboard needed.
- **Mainnet & testnet encodings** — WIF and XPRV can be encoded for testnet (clearly banner-labeled).
- **Save derivations on your terms** — to private Internal storage by default, or to the USB-visible Airlock behind an explicit warning, with a built-in browser to view and delete saved files.
- **Bare-seed root, always** — children derive from the master seed itself; BIP-39 passphrases never enter the derivation (KeyOS exposes base entropy only), so a wallet that folds an active BIP-39 passphrase into its BIP-85 children will derive a different family. The derive screen says so, right above the button.
- **Secrets stay secret** — nothing sensitive ever appears in logs, and everything happens on a device with no network stack.

> The screenshots show children of the **all-zero test seed** ("abandon … art") in the simulator — publicly known vectors, never funded.

## Why there is no installable release yet

The app is complete and verified in the simulator, but **KeyOS 1.4 / SDK
1.0.0 reserves the `GetSeed` permission for Foundation-signed apps** — and
deriving from the device master seed is this app's whole purpose. A
third-party-signed sideload cannot hold that permission today, so we are
not publishing an installable build that would fail on first use. An
installable release will follow as soon as the platform allows it.

## Get it running

With the Foundation SDK installed, build and launch in the simulator with:

```bash
foundation sim
```

## Learn more

- [THIRD-PARTY.md](THIRD-PARTY.md) — libraries this app is built on

## Support

If this app is useful to you, a small bitcoin donation is always appreciated — entirely optional.

<div align="center">

<img src="donate-qr.png" alt="Donate bitcoin" width="200">

**`bc1qkmg7qek6vuuw6hqp9sm06krzcr7pwd5jhcr43f`**

</div>

Donations help cover development costs and keep more open-source bitcoin tools coming. No VC funding, no ads, no tracking.

## License & disclaimer

Licensed under the GNU General Public License v3.0 or later — see [COPYING](COPYING). Sections 15–17 of that license disclaim all warranty and limit liability; the notes below restate that in plain language.

The **`bip85-core/`** library inside this repository carries its own, more permissive terms: **MIT OR Apache-2.0**, see [`bip85-core/LICENSE-MIT`](bip85-core/LICENSE-MIT) and [`bip85-core/LICENSE-APACHE`](bip85-core/LICENSE-APACHE). It holds the BIP-85 derivation, and the split is deliberate — other projects, including non-GPL peers of this app, are meant to build on that crate. The GPL above covers the application around it.

This is experimental software and it has **not been independently audited**.
It is provided **"as is", without warranty of any kind**, express or implied,
including but not limited to the warranties of merchantability, fitness for a
particular purpose, and non-infringement.

**Use it at your own risk.** To the maximum extent permitted by law, in no
event shall the authors, copyright holders, or contributors be liable for any
claim, damages, or other liability — including, without limitation,
**loss of bitcoin or other funds, loss of keys or seeds, or loss of data** — whether in an action of contract, tort, or
otherwise, arising from, out of, or in connection with this software or its
use.

Nothing in this project is financial, investment, legal, or tax advice. You
are solely responsible for verifying addresses, amounts, fees, and backups
before moving funds, and for complying with the laws of your jurisdiction.
Test on test networks, or with amounts you can afford to lose, first.

Derived child seeds are **real, spendable secrets**. Anyone who sees a derived mnemonic, WIF, XPRV, or SeedQR can take any funds it controls — treat every derivation with the same care as the device master seed it came from.
