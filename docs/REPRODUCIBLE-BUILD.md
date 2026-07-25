# Reproducible build

Companion to `DISTRIBUTION.md` §3 ("the published binary must be byte-for-byte reproducible from
the public source"). This documents the pinned toolchain, the exact build steps, and what's
actually been verified so far, not what's aspired to.

---

## 1. Pinned toolchain (as used and verified this pass)

| Component | Version |
|---|---|
| Rust (`rustc`/`cargo`) | 1.96.0 (`ac68faa20`, 2026-05-25) |
| `cargo-ndk` | 4.1.2 |
| Android NDK | r27c (`27.2.12479018`) |
| Android Gradle Plugin | 8.7.2 |
| Kotlin | 2.1.20 |
| Gradle | 8.11.1 (via the committed wrapper, `android/gradle/wrapper/gradle-wrapper.properties`) |
| `compileSdk` / `targetSdk` / `minSdk` | 35 / 35 / 26 |

Every Android dependency in `android/app/build.gradle.kts` is pinned to an exact version already
(no `+`/range selectors), confirmed by reading the file, not assumed.

**`rust-toolchain.toml` added** (later pass, pins `channel = "1.96.0"`, verified: `rustup`/`cargo`
resolve it exactly). **Real regression this pin caused, found via an actual on-device QA pass, not
a hypothetical:** `rustup`'s Android cross-compile targets (`aarch64-linux-android` etc.) are
installed per-toolchain, not shared across toolchain identities, pinning to `1.96.0` (a different
identity from the ambient `stable` the targets were originally added under) silently broke
`cargo ndk`'s builds with `error[E0463]: can't find crate for `std``. Worse, this went unnoticed
for two consecutive "successful" rebuilds in the same session, because the invocation piped
through `| tail -N` without `set -o pipefail`, so a failing `cargo ndk` inside the pipe still
reported exit code 0 (`tail`'s own exit code), the app kept shipping a 40-minutes-stale native
library that crashed on launch (`UnsatisfiedLinkError`, missing symbols for that pass's new FFI
exports) while every build step *reported* success. Fixed by reinstalling the targets
(`rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`) and, going
forward, always piping `cargo ndk`'s output through `set -o pipefail` (or checking `${PIPESTATUS[0]}`)
rather than trusting a piped command's reported exit code. **Still not enforced:** no NDK/AGP/Gradle
version check inside the build itself, a contributor with a different NDK wouldn't be
stopped/warned, only `rustc`/`cargo` are pinned so far.

**Kotlin bumped 2.0.21 → 2.1.20** (Betar design-system pass, building the Compose theme layer
under `android/app/src/main/java/india/projectmesh/app/ui/theme/`): 2.0.21 could not compile
against the bumped `androidx.compose:compose-bom` (needed for the newer `material3`), failing
with "internal in file" errors, the signature of a Kotlin/library metadata mismatch, confirmed
by reading the actual dependency graph (`./gradlew :app:dependencyInsight`) rather than guessed.
2.1.20 is what Compose's own transitive dependencies already forced `kotlin-stdlib` to when
resolved under 2.0.21. `assembleDebug` passes clean on 2.1.20. This does not touch or invalidate
the Rust `.so` reproducibility verified above (Kotlin only compiles the `app/` module, not
`core/`), but the "full packaged APK, not just the `.so`" reproducibility check in §4 was already
marked not-done before this change and still is, this bump is one more thing that check will
need to account for whenever it's actually run.

## 2. Build steps

1. Cross-compile the Rust core for all three ABIs (see `android/app/src/main/jniLibs/README.md`
   for the exact command, `cargo ndk -o android/app/src/main/jniLibs -t arm64-v8a -t
   armeabi-v7a -t x86_64 build --release -p mesh-core`).
2. `cd android && ./gradlew assembleRelease` (or `assembleDebug` for an unsigned debug build:
   this project has not set up release signing yet, see §4).

No other inputs. No code generation step depends on network access, wall-clock time, or
machine-specific paths that this pass found (see §3 for how that claim was actually checked, not
just assumed).

## 3. What's actually been verified

**Same-machine determinism, confirmed by direct hash comparison, not assumed:** built the
`arm64-v8a` `.so` twice from a clean `cargo ndk` target directory, from the identical committed
source, on this dev machine, both builds produced byte-for-byte identical output:

```
71fc34bfad7f9160c4890ac2b9ca32525c8421426e8d18835e80adbd001d83db  libmesh_core.so  (build 1)
71fc34bfad7f9160c4890ac2b9ca32525c8421426e8d18835e80adbd001d83db  libmesh_core.so  (build 2)
```

**What this does and doesn't prove, stated plainly:** this confirms the Rust build has no
same-machine non-determinism (no timestamp, PRNG seed, or unstable iteration-order leaking into
the binary), a real and non-trivial property to have gotten right, but **not** the same as F-Droid's
actual reproducibility bar, which requires an *independent* builder (different machine, often a
different OS) to reproduce the exact same output the primary build server published. That
cross-machine/cross-OS test has not been attempted, this dev environment has no second machine
to build on, and no `diffoscope`-based comparison against an F-Droid-side build has been run
(there is no F-Droid-side build yet, see §4).

**Not tested at all:** whether the full signed release APK (Gradle's packaging, resource
compilation, zip entry ordering) is itself reproducible across two builds, only the Rust `.so`
has been checked. APK zip entries commonly carry timestamps that need explicit normalization to
reproduce bit-for-bit; `isMinifyEnabled = false` (already set, see `build.gradle.kts`'s own
comment) removes one common source of build-specific non-determinism (R8/ProGuard's shrinking and
renaming aren't necessarily stable across environments) but doesn't address zip timestamps on its
own.

## 4. What's blocking an actual F-Droid submission, stated honestly

- **`LICENSE` file added** (later pass, AGPL-3.0-or-later, matching `Cargo.toml`'s declared
  license and `docs/GOVERNANCE.md`'s recommendation). No longer a gap.
- **Release signing configured** (later pass): `android/app/build.gradle.kts` reads a
  `signingConfigs.release` from `android/keystore.properties`, both gitignored, never committed,
  the key itself (`android/keystore/betar-release.jks`, RSA-4096, self-signed, 30-year validity,
  `CN=Betar` with no individual named per the project's attribution rule) lives only on this
  machine. `./gradlew assembleRelease` produces a real `CN=Betar`-signed APK now, verified with
  `apksigner verify --print-certs`, not a debug certificate. **Real, unrelated bug hit and fixed
  along the way:** `assembleRelease`'s mandatory `lintVitalAnalyzeRelease` check crashed
  (`IncompatibleClassChangeError` in `NonNullableMutableLiveDataDetector`, AGP 8.7.2's bundled
  lint against the newer Kotlin 2.1.20 analysis-api shape, this project doesn't even use
  LiveData), fixed by disabling that one detector (`lint { disable += "NullSafeMutableLiveData" }`),
  the exact workaround the crash message itself suggested. **What this does not solve:** the
  signing key exists on one machine only, with no backup, no secondary custodian, and no key
  rotation plan, losing it means Betar can never be updated under the same identity again on any
  channel that expects a consistent signature (F-Droid signs its own builds with its own key
  regardless, so that channel is unaffected either way).
- **No actual `fdroiddata` submission.** The metadata this pass adds (`metadata/` at the repo
  root) is F-Droid-build-recipe metadata, reasoned from the publicly documented `fdroiddata` YAML
  schema, **not validated against F-Droid's own linter/build server**, since that requires
  actually submitting a merge request to the separate `fdroiddata` repository and running F-Droid's
  own CI, neither of which is possible from this dev session. Treat it as a well-reasoned starting
  point for whoever does that submission, not a verified-working recipe.
- **Cross-machine reproducibility untested**, per §3.
