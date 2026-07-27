# Localization, Accessibility, and UX

Companion to `WHITEPAPER.md` §10. The user base is genuinely Indian: many languages, wide
literacy range, and inexpensive hardware. The design treats these as first-order constraints,
not afterthoughts.

---

## 1. Language

- **Many Indic languages, done properly.** Not "English with a translated string file", full
  support for major Indic **scripts** with correct complex-script shaping (conjuncts, matras,
  reordering). Use open **Noto** (or equivalent libre) fonts bundled with the app so rendering
  does not depend on the device's font coverage.
- **Default shipped language: English only, for now.** Bengali is paused (per-user decision,
  2026-07-26, see `PROGRESS.md`), not dropped: the `values-bn/strings.xml` translation still
  exists in the repo and the app's own language pickers still carry the Bengali row commented
  out rather than deleted, so re-enabling it is uncommenting, not redoing the work. Until then,
  the app builds with `defaultConfig.resourceConfigurations += listOf("en")`
  (`android/app/build.gradle.kts`), so Bengali is not actually packaged into the APK even though
  its source strings remain. Every language beyond English, Bengali, Hindi, Assamese, Bodo,
  Ladakhi/Bhoti, Meitei, Nepali, Tamil, Telugu, Marathi, Odia and so on, is community-contributed:
  it ships only once a contributor provides a real translation, not as part of the default
  install.
- **Right-to-left** and mixed-script handling where relevant (e.g. Urdu), if and when that
  language arrives as a community contribution.
- Every language past the two defaults is **community-contributed** through the open
  localization workflow, with a glossary so safety-critical terms (SOS, help, water, danger)
  are consistent.

## 2. Low-literacy first

Reading fluency must **not** be a prerequisite for calling for help or understanding a bulletin.

- **Icon-led navigation:** primary actions (SOS, message, map, resources) are large, labelled
  icons.
- **Voice notes** as a first-class input and output, so users can speak and listen instead of
  read and type.
- **Audio prompts / text-to-speech** (offline where a libre engine exists) for key flows.
- **Pictographic categories** for SOS type and resource type (medical cross, water drop, fire,
  shelter), so a tap conveys meaning without words.
- **Minimal text, high contrast, large targets.**

## 3. Low-end devices

India's volume market is inexpensive Android phones. Targets:

- **≈2 GB RAM** devices run the app comfortably.
- **Small install size** and a **small persistent footprint** (bounded envelope store,
  configurable).
- **Battery discipline:** scanning/relaying is the main cost; adaptive duty-cycling, a clear
  battery-usage indicator, and a user-controlled **"relay mode"** toggle so users decide when to
  spend battery helping the mesh.
- **OEM battery-manager guidance:** detect MIUI/ColorOS/realme/Samsung background killers and
  walk the user through the exact whitelisting steps needed for reliable background relay.
- **Old Android versions:** support a low minimum API level consistent with the target market.

## 4. Offline-first UX

- **No screen ever blocks on a network call.** Every interaction assumes a device that may never
  touch the internet.
- **Connectivity indicators reflect reality:** the UI shows mesh state (peers nearby, relay
  active, LoRa gateway reachable) rather than a misleading "online/offline" binary.
- **Delivery is shown as it truly is:** "sent to mesh," "carried," "delivered" (on ACK), never a
  false real-time "read" guarantee that the DTN model cannot provide.

## 5. Safety-aware UX

- **Location is opt-in and coarse by default**, with clear control, because precise location can
  endanger users in some scenarios.
- **Relay mode is explicit and visible**, so users always know when their device is
  participating and spending battery.
- **Panic / duress** controls (see `CRYPTOGRAPHY.md`) are reachable but guarded against accidental
  triggering.
- **Plain-language privacy explanations** in the user's language: what the app does and does not
  know, and its limits, drawn from the threat model.

## 6. Trust and onboarding

- **No account, no phone number.** Onboarding is: pick a display name, generate keys (automatic),
  done.
- **In-person verification** via QR / safety string is presented as a simple "scan to add a
  trusted contact" flow, not as cryptographic jargon.
- First-run explains, simply, that the app works without internet and how to help the mesh.
