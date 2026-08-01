//! Structural contract tests for the vendored KeyOS `getrandom` patch.
//!
//! Prompted by a 2026 public disclosure of an RNG failure in a
//! shipped hardware wallet's firmware: bug 1 was a
//! deterministic PRNG silently linked in place of the hardware TRNG. A
//! statistical battery can NEVER catch that class of bug — a fixed-seed
//! CSPRNG is statistically perfect. The only defence is structural:
//! prove the RNG we think we linked is the one we linked.
//!
//! `prime-bip85` has no RNG in its own key path — every BIP-85 child is
//! deterministically derived from the device master seed via `GetSeed`
//! (see `bip85-core/tests/spec_vectors.rs` and `determinism.rs`). But the
//! app still carries `[patch.crates-io] getrandom -> vendor/getrandom`
//! for anything downstream that calls `getrandom()`, so it must carry the
//! same structural guards as its siblings: if that patch ever silently
//! stopped applying, this suite must fail rather than quietly link a
//! different (and possibly non-hardware, possibly deterministic) RNG.
//!
//! Each of the five guards below is written as a pure function over a
//! text or graph input, with a thin wrapper that feeds it the real repo
//! files / real `cargo metadata` output. That split is deliberate: a
//! `mutation_*` test beside each pure function feeds it a deliberately
//! broken input and asserts it FAILS, so the contract can never quietly
//! degrade into a test that cannot fail.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bip85-core has a parent directory (the app root)")
}

// =======================================================================
// 1. Cargo.toml redirects getrandom to the vendored crate via
//    [patch.crates-io].
// =======================================================================

/// Extract the body of a `[section]` (lines between its header and the
/// next top-level `[...]` header, or EOF).
fn extract_toml_section(toml: &str, section: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut found = false;
    let mut body = String::new();
    for line in toml.lines() {
        if found {
            if line.trim_start().starts_with('[') {
                break;
            }
            body.push_str(line);
            body.push('\n');
        } else if line.trim() == header {
            found = true;
        }
    }
    found.then_some(body)
}

/// True if `cargo_toml` has a `[patch.crates-io]` section that redirects
/// `getrandom` to a `vendor/getrandom` path.
fn patches_getrandom_to_vendor(cargo_toml: &str) -> bool {
    match extract_toml_section(cargo_toml, "patch.crates-io") {
        Some(body) => body.lines().any(|line| {
            let line = line.trim();
            line.starts_with("getrandom") && line.contains("vendor/getrandom")
        }),
        None => false,
    }
}

#[test]
fn cargo_toml_patches_getrandom_to_vendor() {
    let cargo_toml = fs::read_to_string(repo_root().join("Cargo.toml"))
        .expect("repo Cargo.toml reads");
    assert!(
        patches_getrandom_to_vendor(&cargo_toml),
        "Cargo.toml must have a [patch.crates-io] section redirecting \
         getrandom to vendor/getrandom"
    );
}

#[test]
fn mutation_patch_section_present_is_detected() {
    let toml = "[package]\nname = \"x\"\n\n[patch.crates-io]\n\
                getrandom = { path = \"vendor/getrandom\" }\n";
    assert!(patches_getrandom_to_vendor(toml));
}

#[test]
fn mutation_missing_patch_section_fails() {
    let toml = "[package]\nname = \"x\"\n\n[dependencies]\nfoo = \"1\"\n";
    assert!(!patches_getrandom_to_vendor(toml));
}

#[test]
fn mutation_patch_pointing_elsewhere_fails() {
    let toml = "[patch.crates-io]\ngetrandom = { path = \"vendor/some-other-crate\" }\n";
    assert!(!patches_getrandom_to_vendor(toml));
}

#[test]
fn mutation_getrandom_redirect_outside_patch_section_fails() {
    // The same line, but living under [dependencies] instead of
    // [patch.crates-io] — a normal dependency override does NOT replace
    // every transitive user of crates.io's getrandom.
    let toml = "[dependencies]\ngetrandom = { path = \"vendor/getrandom\" }\n";
    assert!(!patches_getrandom_to_vendor(toml));
}

#[test]
fn mutation_unrelated_patch_entry_does_not_satisfy_it() {
    let toml = "[patch.crates-io]\nsome-other-crate = { path = \"vendor/some-other-crate\" }\n";
    assert!(!patches_getrandom_to_vendor(toml));
}

// =======================================================================
// 2. lib.rs: cfg(keyos) precedes feature = "custom" in the backend
//    cfg_if! chain, and the final fallback arm is compile_error!.
// =======================================================================

/// Extract the body of the (first) `cfg_if! { ... }` macro call, matching
/// braces so nested `{ }` inside each arm doesn't truncate it early.
fn extract_cfg_if_block(src: &str) -> Option<&str> {
    let marker = "cfg_if! {";
    let start = src.find(marker)? + marker.len();
    let bytes = src.as_bytes();
    let mut depth = 1i32;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[start..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// True if `cfg(keyos)` appears before `feature = "custom"` in the block.
/// Ordering is load-bearing: cfg_if! takes the FIRST arm whose predicate
/// holds, so if `custom` ever won, a device build missing `--cfg keyos`
/// would silently rebind to a different backend instead of failing to
/// build (today it fails closed with "target is not supported").
fn keyos_arm_precedes_custom_arm(cfg_if_block: &str) -> bool {
    match (cfg_if_block.find("cfg(keyos)"), cfg_if_block.find("feature = \"custom\"")) {
        (Some(keyos), Some(custom)) => keyos < custom,
        _ => false,
    }
}

/// True if the block's final, unconditional `} else { ... }` arm (as
/// opposed to every preceding `} else if #[cfg(...)] { ... }` arm) leads
/// with `compile_error!`.
fn final_fallback_is_compile_error(cfg_if_block: &str) -> bool {
    let marker = "} else {";
    match cfg_if_block.rfind(marker) {
        Some(idx) => cfg_if_block[idx + marker.len()..]
            .trim_start()
            .starts_with("compile_error!"),
        None => false,
    }
}

#[test]
fn lib_rs_keyos_precedes_custom_with_compile_error_fallback() {
    let lib_rs = fs::read_to_string(repo_root().join("vendor/getrandom/src/lib.rs"))
        .expect("vendor/getrandom/src/lib.rs reads");
    let block = extract_cfg_if_block(&lib_rs)
        .expect("a cfg_if! { … } block must exist in vendor/getrandom/src/lib.rs");
    assert!(
        keyos_arm_precedes_custom_arm(block),
        "cfg(keyos) arm must appear before the feature = \"custom\" arm"
    );
    assert!(
        final_fallback_is_compile_error(block),
        "the cfg_if! chain's final arm must be compile_error!, not a silent fallback"
    );
}

const REAL_SHAPE_CFG_IF: &str = r#"
    if #[cfg(target_os = "linux")] {
        mod linux;
    } else if #[cfg(keyos)] {
        #[path = "xous.rs"] mod imp;
    } else if #[cfg(feature = "custom")] {
        use custom as imp;
    } else {
        compile_error!("target is not supported");
    }
"#;

#[test]
fn mutation_real_shaped_block_passes_both_checks() {
    assert!(keyos_arm_precedes_custom_arm(REAL_SHAPE_CFG_IF));
    assert!(final_fallback_is_compile_error(REAL_SHAPE_CFG_IF));
}

#[test]
fn mutation_custom_arm_reordered_before_keyos_fails() {
    let broken = r#"
    if #[cfg(feature = "custom")] {
        use custom as imp;
    } else if #[cfg(keyos)] {
        #[path = "xous.rs"] mod imp;
    } else {
        compile_error!("target is not supported");
    }
"#;
    assert!(!keyos_arm_precedes_custom_arm(broken));
}

#[test]
fn mutation_missing_keyos_arm_fails() {
    let broken = r#"
    if #[cfg(feature = "custom")] {
        use custom as imp;
    } else {
        compile_error!("target is not supported");
    }
"#;
    assert!(!keyos_arm_precedes_custom_arm(broken));
}

#[test]
fn mutation_fallback_silently_rebinding_to_custom_fails() {
    // This is the exact bug the check exists to catch: a device build
    // missing --cfg keyos would silently pick up `custom` instead of
    // refusing to compile.
    let broken = r#"
    if #[cfg(keyos)] {
        #[path = "xous.rs"] mod imp;
    } else if #[cfg(feature = "custom")] {
        use custom as imp;
    } else {
        use custom as imp;
    }
"#;
    assert!(!final_fallback_is_compile_error(broken));
}

#[test]
fn mutation_no_bare_else_at_all_fails() {
    let broken = r#"
    if #[cfg(keyos)] {
        #[path = "xous.rs"] mod imp;
    } else if #[cfg(feature = "custom")] {
        use custom as imp;
    }
"#;
    assert!(!final_fallback_is_compile_error(broken));
}

// =======================================================================
// 3. xous.rs still calls the fill-verification helpers.
// =======================================================================

/// True if all three fill-verification helpers are still called (not
/// merely imported) from the KeyOS TRNG backend.
fn calls_fill_verification_helpers(xous_src: &str) -> bool {
    ["write_sentinel(", "looks_unfilled(", "words_for("]
        .iter()
        .all(|call| xous_src.contains(call))
}

#[test]
fn xous_rs_still_calls_fill_verification_helpers() {
    let xous_rs = fs::read_to_string(repo_root().join("vendor/getrandom/src/xous.rs"))
        .expect("vendor/getrandom/src/xous.rs reads");
    assert!(
        calls_fill_verification_helpers(&xous_rs),
        "xous.rs must still call write_sentinel, looks_unfilled and words_for"
    );
}

#[test]
fn mutation_all_three_calls_present_passes() {
    let src = "write_sentinel(buf); let n = words_for(len); if looks_unfilled(buf) {}";
    assert!(calls_fill_verification_helpers(src));
}

#[test]
fn mutation_missing_write_sentinel_call_fails() {
    let src = "let n = words_for(len); if looks_unfilled(buf) {}";
    assert!(!calls_fill_verification_helpers(src));
}

#[test]
fn mutation_missing_looks_unfilled_call_fails() {
    let src = "write_sentinel(buf); let n = words_for(len);";
    assert!(!calls_fill_verification_helpers(src));
}

#[test]
fn mutation_missing_words_for_call_fails() {
    let src = "write_sentinel(buf); if looks_unfilled(buf) {}";
    assert!(!calls_fill_verification_helpers(src));
}

#[test]
fn mutation_import_without_call_is_not_enough() {
    // Importing the helpers but never calling them (e.g. a revert that
    // keeps the `use` line but drops the verification logic) must fail.
    let src = "use crate::trng_check::{looks_unfilled, words_for, write_sentinel};";
    assert!(!calls_fill_verification_helpers(src));
}

// =======================================================================
// 4. register_custom_getrandom! appears nowhere in the repo outside
//    vendor/getrandom.
// =======================================================================

/// Walk `root`, skipping `target/`, `.git/`, and symlinks (the latter so
/// the walk can never follow CLAUDE.md/NOTES.md/DEVELOPMENT.md out of the
/// repo, and can never loop). Returns (repo-relative path, content) for
/// every file that decodes as UTF-8 (binary assets are silently skipped —
/// they cannot contain valid Rust source).
fn collect_repo_files(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else { continue };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if file_type.is_dir() {
                if name == "target" || name == ".git" {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, content));
            }
        }
    }
    out
}

/// Repo-relative paths (outside `vendor/getrandom/`) whose content
/// invokes `register_custom_getrandom!`. `excluded` is an extra set of
/// exact paths to skip — used by the real-file wrapper below to exempt
/// this very test file, which legitimately spells the macro name in its
/// own mutation-test string fixtures.
fn find_offending_files(files: &[(String, String)], excluded: &[&str]) -> Vec<String> {
    files
        .iter()
        .filter(|(path, _)| !path.starts_with("vendor/getrandom/"))
        .filter(|(path, _)| !excluded.contains(&path.as_str()))
        .filter(|(_, content)| content.contains("register_custom_getrandom!"))
        .map(|(path, _)| path.clone())
        .collect()
}

#[test]
fn register_custom_getrandom_only_used_inside_vendor() {
    let files = collect_repo_files(repo_root());
    assert!(!files.is_empty(), "sanity: the repo file walk found nothing");
    // Sanity: the walk must actually reach vendor/getrandom, or the
    // exclusion filter below would vacuously pass by never seeing the
    // macro at all.
    assert!(
        files.iter().any(|(p, c)| p.starts_with("vendor/getrandom/")
            && c.contains("register_custom_getrandom!")),
        "sanity: expected vendor/getrandom/src/custom.rs to define/use \
         register_custom_getrandom! — the walk or the fixture is broken"
    );
    // This file itself (mutation fixtures below) legitimately contains
    // the macro name as a string, not a use of it — exempt it by exact
    // path rather than weakening the substring match for everyone else.
    let offenders = find_offending_files(&files, &["bip85-core/tests/rng_backend.rs"]);
    assert!(
        offenders.is_empty(),
        "register_custom_getrandom! used outside vendor/getrandom: {offenders:?}"
    );
}

#[test]
fn mutation_macro_only_inside_vendor_is_clean() {
    let files = vec![
        (
            "vendor/getrandom/src/custom.rs".to_string(),
            "macro_rules! register_custom_getrandom { () => {} }".to_string(),
        ),
        ("src/main.rs".to_string(), "fn main() {}".to_string()),
    ];
    assert!(find_offending_files(&files, &[]).is_empty());
}

#[test]
fn mutation_macro_use_outside_vendor_is_flagged() {
    let files = vec![
        (
            "vendor/getrandom/src/custom.rs".to_string(),
            "macro_rules! register_custom_getrandom { () => {} }".to_string(),
        ),
        (
            "src/main.rs".to_string(),
            "register_custom_getrandom!(always_fail);".to_string(),
        ),
    ];
    assert_eq!(
        find_offending_files(&files, &[]),
        vec!["src/main.rs".to_string()]
    );
}

#[test]
fn mutation_excluded_path_is_not_flagged_but_others_still_are() {
    let files = vec![
        (
            "bip85-core/tests/rng_backend.rs".to_string(),
            "register_custom_getrandom!(fixture);".to_string(),
        ),
        (
            "src/main.rs".to_string(),
            "register_custom_getrandom!(always_fail);".to_string(),
        ),
    ];
    assert_eq!(
        find_offending_files(&files, &["bip85-core/tests/rng_backend.rs"]),
        vec!["src/main.rs".to_string()]
    );
}

// =======================================================================
// 5. Dependency-graph guard: no crate reachable through NORMAL
//    (non-dev) dependencies from the app root pulls a getrandom other
//    than the vendored 0.2.x.
// =======================================================================

/// True if this edge's `dep_kinds` includes at least one entry that is
/// Normal (`kind: null`) or `"build"` AND unconditional (`target: null`).
/// `cargo metadata` represents dev-only edges as `kind: "dev"` and
/// platform/`cfg`-gated edges (e.g. `cfg(windows)`) with a non-null
/// `target`; both are excluded here.
///
/// Excluding dev edges is exact. Excluding **every** target-gated edge is
/// NOT: `cargo metadata` without `--filter-platform` reports each gated
/// edge unconditionally, so keeping them reaches the entire lockfile
/// (726 packages here, `getrandom` 0.3.4/0.4.2 included) and the guard
/// would be useless. Dropping them leaves a known blind spot — a
/// `getrandom` gated behind a cfg that is TRUE on
/// `armv7a-unknown-xous-elf` would not be flagged. That is deliberate,
/// and it is the weaker half of this check.
///
/// The exact version is `cargo metadata --filter-platform
/// armv7a-unknown-xous-elf`, which needs the KeyOS target spec and so
/// only resolves inside the SDK's nix shell. Reach for it if a dependency
/// ever does gate an RNG on the device target; the realistic regression
/// this guard exists to catch — a bump to a crate using `rand 0.9`, whose
/// `getrandom 0.3` edge is unconditional — is caught as written.
fn dep_kind_reaches_device(dep_kinds: &[Value]) -> bool {
    dep_kinds.iter().any(|k| {
        let kind_ok = k["kind"].is_null() || k["kind"].as_str() == Some("build");
        let unconditional = k["target"].is_null();
        kind_ok && unconditional
    })
}

/// BFS over `metadata.resolve.nodes` from `metadata.resolve.root`,
/// following only edges `dep_kind_reaches_device` accepts. Returns every
/// reached package id (including the root).
fn reachable_package_ids(metadata: &Value) -> HashSet<String> {
    let root = metadata["resolve"]["root"].as_str().unwrap_or_default().to_string();
    let nodes = metadata["resolve"]["nodes"].as_array().cloned().unwrap_or_default();
    let mut by_id: HashMap<String, Value> = HashMap::new();
    for n in nodes {
        if let Some(id) = n["id"].as_str() {
            by_id.insert(id.to_string(), n.clone());
        }
    }

    let mut seen = HashSet::new();
    let mut stack = vec![root];
    while let Some(cur) = stack.pop() {
        if !seen.insert(cur.clone()) {
            continue;
        }
        let Some(node) = by_id.get(&cur) else { continue };
        let Some(deps) = node["deps"].as_array() else { continue };
        for dep in deps {
            let Some(kinds) = dep["dep_kinds"].as_array() else { continue };
            if dep_kind_reaches_device(kinds) {
                if let Some(pkg) = dep["pkg"].as_str() {
                    stack.push(pkg.to_string());
                }
            }
        }
    }
    seen
}

/// `getrandom`'s crate name from a cargo `PackageId` string. IDs look
/// like `registry+https://…#getrandom@0.3.4` (explicit name@version) or
/// `path+file:///…/vendor/getrandom#0.2.10` (name omitted when it matches
/// the directory the path dependency points at).
fn package_name(pkg_id: &str) -> &str {
    let after_hash = pkg_id.rsplit('#').next().unwrap_or(pkg_id);
    if let Some((name, _version)) = after_hash.rsplit_once('@') {
        name
    } else {
        let before_len = pkg_id.len() - after_hash.len();
        let before_hash = pkg_id[..before_len].trim_end_matches('#');
        before_hash.rsplit('/').next().unwrap_or(before_hash)
    }
}

/// Reachable `getrandom` package ids that are NOT the vendored path dep.
fn non_vendor_getrandom_ids(metadata: &Value) -> Vec<String> {
    reachable_package_ids(metadata)
        .into_iter()
        .filter(|id| package_name(id) == "getrandom")
        .filter(|id| !id.contains("vendor/getrandom"))
        .collect()
}

fn real_cargo_metadata() -> Value {
    let manifest_path = repo_root().join("Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .arg("--manifest-path")
        .arg(&manifest_path)
        .output()
        .expect("failed to spawn `cargo metadata`");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Defensive: some shells print banner text on stdout before a command
    // runs (observed with `nix develop --command`); skip to the first
    // '{' so a stray banner line can't break parsing.
    let json_start = stdout
        .find('{')
        .expect("cargo metadata stdout contains no JSON object");
    serde_json::from_str(&stdout[json_start..]).expect("cargo metadata produced invalid JSON")
}

#[test]
fn dependency_graph_reaches_only_vendored_getrandom() {
    let metadata = real_cargo_metadata();
    let reachable = reachable_package_ids(&metadata);
    // Sanity: the vendored getrandom itself must be reachable, or this
    // test would vacuously pass by never walking far enough.
    assert!(
        reachable
            .iter()
            .any(|id| package_name(id) == "getrandom" && id.contains("vendor/getrandom")),
        "sanity: the vendored getrandom package should be reachable from the app root"
    );
    let bad = non_vendor_getrandom_ids(&metadata);
    assert!(
        bad.is_empty(),
        "non-vendored getrandom reachable via a normal/build dependency: {bad:?} — \
         a bump somewhere in the graph silently bypassed the TRNG patch"
    );
}

#[test]
fn mutation_package_name_parses_registry_and_path_ids() {
    assert_eq!(
        package_name("registry+https://github.com/rust-lang/crates.io-index#getrandom@0.3.4"),
        "getrandom"
    );
    assert_eq!(
        package_name("path+file:///Users/x/prime-bip85/vendor/getrandom#0.2.10"),
        "getrandom"
    );
    assert_eq!(
        package_name("path+file:///Users/x/prime-bip85/bip85-core#0.1.0"),
        "bip85-core"
    );
}

#[test]
fn mutation_unconditional_non_vendor_getrandom_is_flagged() {
    let metadata = json!({
        "resolve": {
            "root": "root#0.1.0",
            "nodes": [
                {
                    "id": "root#0.1.0",
                    "deps": [{
                        "pkg": "registry+x#getrandom@0.3.4",
                        "dep_kinds": [{"kind": null, "target": null}]
                    }]
                },
                {"id": "registry+x#getrandom@0.3.4", "deps": []}
            ]
        }
    });
    assert_eq!(non_vendor_getrandom_ids(&metadata), vec!["registry+x#getrandom@0.3.4".to_string()]);
}

#[test]
fn mutation_build_kind_unconditional_getrandom_is_flagged() {
    let metadata = json!({
        "resolve": {
            "root": "root#0.1.0",
            "nodes": [
                {
                    "id": "root#0.1.0",
                    "deps": [{
                        "pkg": "registry+x#getrandom@0.3.4",
                        "dep_kinds": [{"kind": "build", "target": null}]
                    }]
                },
                {"id": "registry+x#getrandom@0.3.4", "deps": []}
            ]
        }
    });
    assert_eq!(non_vendor_getrandom_ids(&metadata).len(), 1);
}

#[test]
fn mutation_target_gated_getrandom_is_not_flagged() {
    // Mirrors the real jobserver -> getrandom 0.3.4 edge (cfg(windows)):
    // present in Cargo.lock today, never reachable on device or macOS.
    let metadata = json!({
        "resolve": {
            "root": "root#0.1.0",
            "nodes": [
                {
                    "id": "root#0.1.0",
                    "deps": [{
                        "pkg": "registry+x#getrandom@0.3.4",
                        "dep_kinds": [{"kind": null, "target": "cfg(windows)"}]
                    }]
                },
                {"id": "registry+x#getrandom@0.3.4", "deps": []}
            ]
        }
    });
    assert!(non_vendor_getrandom_ids(&metadata).is_empty());
}

#[test]
fn mutation_dev_only_getrandom_is_not_flagged() {
    let metadata = json!({
        "resolve": {
            "root": "root#0.1.0",
            "nodes": [
                {
                    "id": "root#0.1.0",
                    "deps": [{
                        "pkg": "registry+x#getrandom@0.4.2",
                        "dep_kinds": [{"kind": "dev", "target": null}]
                    }]
                },
                {"id": "registry+x#getrandom@0.4.2", "deps": []}
            ]
        }
    });
    assert!(non_vendor_getrandom_ids(&metadata).is_empty());
}

#[test]
fn mutation_vendored_getrandom_never_flagged() {
    let metadata = json!({
        "resolve": {
            "root": "root#0.1.0",
            "nodes": [
                {
                    "id": "root#0.1.0",
                    "deps": [{
                        "pkg": "path+file:///repo/vendor/getrandom#0.2.10",
                        "dep_kinds": [{"kind": null, "target": null}]
                    }]
                },
                {"id": "path+file:///repo/vendor/getrandom#0.2.10", "deps": []}
            ]
        }
    });
    assert!(non_vendor_getrandom_ids(&metadata).is_empty());
}

#[test]
fn mutation_transitive_reachability_through_two_hops_is_flagged() {
    // root -> mid -> getrandom, unconditional both hops: a bump two
    // levels deep must still be caught, not just a direct dependency.
    let metadata = json!({
        "resolve": {
            "root": "root#0.1.0",
            "nodes": [
                {
                    "id": "root#0.1.0",
                    "deps": [{"pkg": "mid#0.1.0", "dep_kinds": [{"kind": null, "target": null}]}]
                },
                {
                    "id": "mid#0.1.0",
                    "deps": [{
                        "pkg": "registry+x#getrandom@0.3.4",
                        "dep_kinds": [{"kind": null, "target": null}]
                    }]
                },
                {"id": "registry+x#getrandom@0.3.4", "deps": []}
            ]
        }
    });
    assert_eq!(non_vendor_getrandom_ids(&metadata).len(), 1);
}

#[test]
fn mutation_only_reachable_via_dev_edge_is_not_flagged_even_if_a_normal_edge_exists_deeper() {
    // root -[dev]-> mid -[normal]-> getrandom: `mid` itself is only
    // reached via a dev-dependency edge, so nothing beneath it should be
    // walked at all, regardless of what kind of edge `mid` uses further
    // down.
    let metadata = json!({
        "resolve": {
            "root": "root#0.1.0",
            "nodes": [
                {
                    "id": "root#0.1.0",
                    "deps": [{"pkg": "mid#0.1.0", "dep_kinds": [{"kind": "dev", "target": null}]}]
                },
                {
                    "id": "mid#0.1.0",
                    "deps": [{
                        "pkg": "registry+x#getrandom@0.3.4",
                        "dep_kinds": [{"kind": null, "target": null}]
                    }]
                },
                {"id": "registry+x#getrandom@0.3.4", "deps": []}
            ]
        }
    });
    assert!(non_vendor_getrandom_ids(&metadata).is_empty());
}
