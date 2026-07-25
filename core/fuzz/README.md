# Wire-parser fuzzing

Six `cargo-fuzz` targets covering every untrusted-bytes-in parser reachable from the mesh (a peer,
or an attacker on the mesh, controls these bytes before any signature/authentication check runs):

| Target | Parses |
|---|---|
| `envelope_from_bytes` | `Envelope::from_bytes`, every relayed frame |
| `bloom_filter_from_bytes` | `BloomFilter::from_bytes`, gossip summary vectors |
| `contact_message_from_bytes` | `ContactMessage::from_bytes`, the outer contact-protocol frame |
| `prekey_bundle_from_bytes` | `PrekeyBundle::from_bytes`, classical X3DH prekey bundles |
| `hybrid_bundle_from_bytes` | `HybridBundle::from_bytes`, the hybrid bundle `DirectMessenger.kt` actually broadcasts |
| `unpack_initiation_message` | `unpack_initiation_message`, the hybrid-bootstrap "first contact" reply |

Run with `cargo +nightly fuzz run <target>` from `core/`.

## Not runnable in this dev environment

**`cargo fuzz run` fails to even build on this Windows/MSVC dev box**, not a fuzzing-found bug,
a toolchain/linker conflict. `mesh-core`'s `[lib] crate-type = ["lib", "staticlib", "cdylib"]`
(the `cdylib` half is what Android's `.so` needs) means Cargo also builds a `cdylib` copy of
`mesh-core` as a side effect of the fuzz binary depending on it, and MSVC's linker refuses that
spurious `cdylib` output because it never satisfies the sanitizer/coverage runtime symbols
libFuzzer's instrumentation injects into every compiled object of the same crate (`main` missing
with `-s address` (Address Sanitizer, the default); `__sanitizer_cov_pcs_init` and friends missing
with `-s none`, i.e. SanitizerCoverage without ASan, tried both, confirmed via the actual linker
errors, not assumed):

```
LINK : error LNK2001: unresolved external symbol main
mesh_core.dll : fatal error LNK1120: 1 unresolved externals
```
```
...rlib(...): error LNK2001: unresolved external symbol __sanitizer_cov_pcs_init
mesh_core.dll : fatal error LNK1120: 17 unresolved externals
```

This is specifically a Windows/MSVC linker-strictness issue: ELF shared objects on Linux don't
need a `main` symbol the same way, so this exact conflict is not expected to reproduce there. The
real fix (splitting `mesh-core` into a pure-`rlib` crate plus a thin `cdylib`-only wrapper crate
for Android, so nothing depending on the rlib ever triggers a `cdylib` build) is a real
architectural change to how the Android `.so` gets built, out of scope for adding a fuzzing
harness, and risks destabilizing the Android build pipeline this project has verified working
multiple times this session. Not attempted.

**How to actually run these:** on Linux or macOS (a contributor's machine, or, the natural home
for continuous fuzzing anyway, a CI job on an `ubuntu-latest`-style runner). Not attempted in
this dev session; no such CI exists yet for this repo.
