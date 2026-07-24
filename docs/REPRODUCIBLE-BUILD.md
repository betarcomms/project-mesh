# Reproducible build

Companion to `DISTRIBUTION.md` §3 ("the published binary must be byte-for-byte reproducible from
the public source"). This documents the pinned toolchain, the exact build steps, and what's
actually been verified so far — not what's aspired to.

---

## 1. Pinned toolchain (as used and verified this pass)

| Component | Version |
|---|---|
| Rust (`rustc`/`cargo`) | 1.96.0 (`ac68faa20`, 2026-05-25) |
| `cargo-ndk` | 4.1.2 |
| Android NDK | r27c (`27.2.12479018`) |
| Android Gradle Plugin | 8.7.2 |
| Kotlin | 2.0.21 |
| Gradle | 8.11.1 (via the committed wrapper — `android/gradle/wrapper/gradle-wrapper.properties`) |
| `compileSdk` / `targetSdk` / `minSdk` | 35 / 35 / 26 |

Every Android dependency in `android/app/build.gradle.kts` is pinned to an exact version already
(no `+`/range selectors) — confirmed by reading the file, not assumed.

**Not yet pinned, a real gap:** nothing in the repo *enforces* these versions (no
`rust-toolchain.toml`, no NDK version check in `build.gradle.kts`). A future contributor building
with a different Rust/NDK/AGP version wouldn't be stopped or warned. Tracked as a follow-up, not
fixed this pass — pinning via `rust-toolchain.toml` is straightforward; enforcing the NDK/AGP/
Gradle versions from within the build is more involved and wasn't attempted here.

## 2. Build steps

1. Cross-compile the Rust core for all three ABIs (see `android/app/src/main/jniLibs/README.md`
   for the exact command — `cargo ndk -o android/app/src/main/jniLibs -t arm64-v8a -t
   armeabi-v7a -t x86_64 build --release -p mesh-core`).
2. `cd android && ./gradlew assembleRelease` (or `assembleDebug` for an unsigned debug build —
   this project has not set up release signing yet, see §4).

No other inputs. No code generation step depends on network access, wall-clock time, or
machine-specific paths that this pass found (see §3 for how that claim was actually checked, not
just assumed).

## 3. What's actually been verified

**Same-machine determinism, confirmed by direct hash comparison, not assumed:** built the
`arm64-v8a` `.so` twice from a clean `cargo ndk` target directory, from the identical committed
source, on this dev machine — both builds produced byte-for-byte identical output:

```
71fc34bfad7f9160c4890ac2b9ca32525c8421426e8d18835e80adbd001d83db  libmesh_core.so  (build 1)
71fc34bfad7f9160c4890ac2b9ca32525c8421426e8d18835e80adbd001d83db  libmesh_core.so  (build 2)
```

**What this does and doesn't prove, stated plainly:** this confirms the Rust build has no
same-machine non-determinism (no timestamp, PRNG seed, or unstable iteration-order leaking into
the binary) — a real and non-trivial property to have gotten right, but **not** the same as F-Droid's
actual reproducibility bar, which requires an *independent* builder (different machine, often a
different OS) to reproduce the exact same output the primary build server published. That
cross-machine/cross-OS test has not been attempted — this dev environment has no second machine
to build on, and no `diffoscope`-based comparison against an F-Droid-side build has been run
(there is no F-Droid-side build yet — see §4).

**Not tested at all:** whether the full signed release APK (Gradle's packaging, resource
compilation, zip entry ordering) is itself reproducible across two builds — only the Rust `.so`
has been checked. APK zip entries commonly carry timestamps that need explicit normalization to
reproduce bit-for-bit; `isMinifyEnabled = false` (already set, see `build.gradle.kts`'s own
comment) removes one common source of build-specific non-determinism (R8/ProGuard's shrinking and
renaming aren't necessarily stable across environments) but doesn't address zip timestamps on its
own.

## 4. What's blocking an actual F-Droid submission, stated honestly

- **No `LICENSE` file at the repository root.** `Cargo.toml`'s `workspace.package.license =
  "AGPL-3.0-or-later"` and `docs/GOVERNANCE.md`'s recommendation are consistent, but F-Droid (and
  GitHub's own license detection) need the actual license text committed, not just a manifest
  field. Not added this pass.
- **No release signing configured.** `build.gradle.kts` has no `signingConfig` for `release` —
  F-Droid signs its own builds with its own key (a different, expected key from any
  developer-signed release), so this specifically blocks a *developer-signed direct APK*, not the
  F-Droid build itself, but is worth having either way for `DISTRIBUTION.md` §2's other channels
  (IzzyOnDroid, direct download).
- **No actual `fdroiddata` submission.** The metadata this pass adds (`metadata/` at the repo
  root) is F-Droid-build-recipe metadata, reasoned from the publicly documented `fdroiddata` YAML
  schema — **not validated against F-Droid's own linter/build server**, since that requires
  actually submitting a merge request to the separate `fdroiddata` repository and running F-Droid's
  own CI, neither of which is possible from this dev session. Treat it as a well-reasoned starting
  point for whoever does that submission, not a verified-working recipe.
- **Cross-machine reproducibility untested**, per §3.
