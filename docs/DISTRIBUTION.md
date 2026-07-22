# Distribution and the De-Googled Requirement

Companion to `WHITEPAPER.md` §11. De-Googling is both a values choice and a robustness
requirement: the app must run where Google's services are absent, blocked, or unwanted.

---

## 1. No proprietary cloud layer

Project MESH uses **no Google Play Services, no Firebase, and no third-party proprietary
cloud**. Concretely:

| Common Google dependency | What we use instead |
|---|---|
| Firebase Cloud Messaging (push) | No server push at all — the mesh is local; local notifications from the foreground service |
| Google Maps SDK | **MapLibre GL Native + OpenStreetMap** offline tiles |
| Play Services location | Android platform location APIs directly (opt-in, coarse) |
| Play Integrity / SafetyNet | None (not needed; would add a Google dependency) |
| Crashlytics / analytics | None, or a self-hostable/opt-in libre crash reporter with no PII |
| Play-only distribution | F-Droid + IzzyOnDroid + direct APK (Play optional) |

Because there is **no server and no push**, the largest reason apps depend on Google
(notifications) simply does not apply.

## 2. Android distribution

- **F-Droid** as the primary channel — the libre app store aligned with a de-Googled audience.
- **IzzyOnDroid** repository for faster updates.
- **Direct signed APK** download from the project site, with published signing-key fingerprints.
- **Optional Google Play** listing built from the *same* source, for reach — never the only or
  privileged channel.
- Runs on **de-Googled ROMs** (GrapheneOS, LineageOS, /e/OS, CalyxOS) and on stock devices
  without Google connectivity.

## 3. Reproducible builds (mandatory for a security tool)

- The published binary must be **byte-for-byte reproducible** from the public source, so any
  third party can verify the release was built from the code it claims — closing the
  supply-chain gap that source-availability alone leaves open.
- F-Droid's reproducible-build verification is used where possible.
- Build inputs (toolchain versions, dependencies) are pinned; the build is documented and
  scripted.

## 4. iOS distribution (stated honestly)

- iOS apps are distributed through the **App Store** (and **TestFlight** for testing). This is an
  **unavoidable centralization point** — Apple controls the channel.
- We document this plainly rather than pretend iOS distribution is decentralized. It affects
  availability (Apple could remove the app), not the on-device security or the offline operation
  of the mesh.

## 5. Update and integrity

- Releases are **signed**; users can verify signatures and reproducible-build attestations.
- No silent auto-update through a proprietary channel is required; F-Droid and direct APK give
  transparent, verifiable updates.

## 6. Why this matters for the mission

- **Runs anywhere:** a de-Googled app works on cheap devices, custom ROMs, and in environments
  with no Google reachability — exactly the rural/disaster/shutdown contexts in scope.
- **No hidden dependency to fail or leak:** every network capability is either local (the mesh)
  or explicitly optional; there is no background beacon to a proprietary cloud.
- **Verifiable trust:** open source + reproducible builds let the security community, not a
  vendor, vouch for the binary.
