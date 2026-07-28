# Betar Transition: everything still to do

Working document. The project is being renamed, repositioned and made public, and this
tracks all of it in one place so nothing gets lost between sessions.

Status of this file: Parts 1 and 2 record what has already been decided, so future work
does not relitigate settled questions. Part 3 is partly done (org created, secret scan
run). Part 5's package-id rename is done in code; icons are not.

---

## Part 1. Decided

| Thing | Decision |
|---|---|
| App name | **Betar** (বেতার), Bengali for wireless, literally *be-* (without) *tar* (wire) |
| Protocol name | **Project Mesh** stays as the name of the protocol and the Rust core |
| GitHub org | **betarcomms** (renamed from `Betar-Communication` 2026-07-29, manually via GitHub web settings — org login rename isn't exposed via the REST API, confirmed by a failed API attempt first). konkomaji is admin. |
| Repo structure | Three repos, see Part 3: `project-mesh` (protocol core, research site), `betar` (full app, code moved in, 4 releases moved in, showcase site live), `betarchat` (chat-only variant, separate maintained fork of `betar`'s code — repo exists, honest placeholder site live, **no app code yet**, blocked on its own logo/design brief. App display name is **Betar Chat**, two words, repo slug is one word, no hyphen) |
| `project-mesh` URL | `https://github.com/betarcomms/project-mesh` (transferred 2026-07-29, old `konkomaji/project-mesh` URL redirects) |
| `betar` URL | `https://github.com/betarcomms/betar` (public, app code + releases + site live) |
| `betarchat` URL | `https://github.com/betarcomms/betarchat` (public, placeholder site only, no app code yet) |
| `betar` website | `https://betarcomms.github.io/betar/` — live, Betar's app showcase site, moved here from `project-mesh` |
| `betarchat` website | `https://betarcomms.github.io/betarchat/` — live, honest single-page placeholder, no logo/design exists yet |
| `project-mesh` website | `https://betarcomms.github.io/project-mesh/` — live, rewritten as a research/protocol site, no longer the app showcase |
| Repo visibility | Going public |
| Betar package id | `app.betar.comm`, replacing `india.projectmesh.app`. Renamed in code. See Part 5 |
| Betar Chat package id | `app.betar.chat`. Not yet a real app, id reserved. See Part 5 |
| Logo | Betar: done, "the wire that is not there", files in `docs/assets/`. Betar Chat: **needs its own separate logo**, not done, see Part 6 |
| Brand blue | `#4BA3E0` |
| Deep blue | `#12608F` (for text and anything small, light blue alone fails contrast) |
| Off white blue ground | `#EEF4F9` |
| Lifted surface | `#F7FAFD` |
| Ink | `#101A22` |
| Emergency red | `#C8102E`, SOS only, never anything else |
| Design system | Material 3 Expressive, light theme primary, dark theme supported |
| Design brief | Written, at `docs/DESIGN-BRIEF.md` |
| Attribution | No individual named anywhere. Repo link only |

### Rejected, so nobody suggests them again

- **Names**: Setu, Sandesh, Jodo, Awaaz, Tarang, Jaal, Chinar, Lantern, Torch, Haven,
  Ripple, Flock, Firefly, Beacon, Bridge. Reasons range from existing app collisions to
  trademark conflicts to connotations that fight the positioning.
- **BetarChat, Betar Chat, Betar App** as names. Chat shrinks the product to its smallest
  part and the org is not an app.
- **The tricolor logo.** Flag colours plus Ashoka Chakra style spokes pull an apolitical
  civic tool toward nationalist coding.
- **Logo concepts** 01 wire into air, 02 released ends, 04 unwound coil. Concept 03 won.

## Part 2. Framing rules, which apply to every file from now on

These govern all documentation, all in app copy and all store listings.

1. **Never call Betar a chat app.** It is communication and safety infrastructure for
   places the network does not reach. Messaging is one of five things it does.
2. **But chat is the first tab and the main screen.** Rule 1 governs words. This governs
   layout. They do not conflict.
3. **No government references anywhere.** No mentions of authorities, censorship,
   circumvention, bans, blocks, protest, activism or surveillance. Outages are described
   as outages and never attributed to anyone.
4. **Frame around remote areas, cyclones, floods and network outages.** That is the real
   use case and it needs no help.
5. **Nobody is named.** No developer byline, no company, no contact person. Attribution
   goes to the project and to the public source.
6. **No em dashes.** Use commas, full stops, colons, brackets.
7. **Write like a person.** Short sentences. Drop the repeated tics currently running
   through the docs: "stated plainly", "honest limit", "not silently skipped". Saying it
   once is honest. Saying it forty times is a verbal habit.

## Part 3. GitHub and repository

Three repos live under the org, not one.

| Repo | Contents |
|---|---|
| `project-mesh` | The Rust protocol core only. The existing repo, transferred in as is. |
| `betar` | The full Android app: chat, SOS, bulletin board, resource board, map pins, everything. Future releases and the app's own website live here too. |
| `betarchat` | Same app, same stack, same everything except stripped to chat only: direct messages, groups, channels. No SOS, no bulletin board, no resource board, no map. App display name is Betar Chat; repo slug is one word, no hyphen. |

**Decided 2026-07-29:** `betarchat` is a **separate maintained fork** of `betar`'s code
— its own full copy of the source with SOS/bulletin/resource/map deleted, maintained
independently going forward. Real double-maintenance cost on every future change to
shared code (messaging, transport, identity); accepted as the tradeoff over a build
flavor or branch.

**Open question, still not decided:** how `betar` (once it's a standalone repo) gets the
Rust core it depends on at build time. Today `android/`'s build reaches into `../core`
in the same mono-repo (see `metadata/app.betar.comm.yml`'s `cd ../core && cargo ndk...`
prebuild step). Splitting `android/` out into `betar` breaks that relative path.

**Decided 2026-07-29: git submodule.** `betar` carries `project-mesh` as a submodule for
`core/`, pinned to an exact commit. Rejected the other two: a vendored copy duplicates a
crypto/protocol core across repos (real drift risk on security-relevant code, and the
same double-maintenance cost already accepted once for Betar Chat — not worth taking
twice); a published/prebuilt artifact conflicts with the F-Droid plan already committed
to in Part 7 — F-Droid builds from source on its own server (the existing
`cd ../core && cargo ndk...` prebuild step is that model already working), and this repo
deliberately never commits the compiled `.so` (gitignored, regenerated every build).
Submodule keeps one source of truth, pins `betar` to a reproducible commit, and F-Droid
natively supports it (`submodules: true` in the build recipe).

**Done 2026-07-29, same session as the decision.** Code actually moved:

- `android/` split out of this mono-repo into `betar` via `git subtree split --prefix=android`
  (preserves the 75 commits of history that touched `android/`), pushed as `betar`'s `main`.
- `core/` added to `betar` as a git submodule pointing at `betarcomms/project-mesh`.
- `build.gradle.kts`'s `rootProject.name` and the jniLibs `README.md`'s `cargo ndk` command
  updated for the new layout (`cd core && cargo ndk -o ../app/src/main/jniLibs...`, no more
  `../core`).
- `.gitignore` added to `betar` — the mono-repo's root `.gitignore` covered `android/`'s
  build artifacts and secrets but lived outside the `android/` prefix, so the subtree split
  didn't carry it. Without this, `local.properties`/`.gradle/`/`build/`/keystore files had
  no protection against accidental commit. Fixed before anything got committed on top.
- `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE` added — plain human-written docs,
  per the policy decided above.
- Verified with a **fresh standalone clone** (not just the working tree that did the split):
  `git clone` + `git submodule update --init` + `./gradlew :app:compileDebugKotlin` →
  BUILD SUCCESSFUL. This is the real proof the split works, not just that the split
  commands ran without erroring.
- `betar` scanned with `gitleaks` before going public (same discipline as `project-mesh`):
  one hit, the same known false positive (`joined_passphrases_wrapped_b64`, a
  SharedPreferences key name, not a secret). Filename check for keystore/local.properties/
  `.so` binaries clean. Made public.
- The 4 existing GitHub releases (`v0.1.0`–`v0.1.3-prealpha`, APKs included) moved from
  `project-mesh` to `betar`: same tag names, same release notes (links fixed to point at
  whichever repo the linked doc actually lives in), same APK assets, retagged at the
  commit in `betar`'s subtree-split history matching the original release commit's message.
  Deleted from `project-mesh` after confirming they were live on `betar`.

`project-mesh` already exists on `betarcomms/project-mesh` (transferred, see below) and
is the repo the steps below apply to. `betar` and `betarchat` repos are **created on
GitHub, private, empty** — Part 5's code rename (package id
`app.betar.comm`/`app.betar.chat`) is done and the core-dependency strategy is now
decided, so `betar` is unblocked to actually receive code; no code has been pushed into
either new repo yet.

Do these in order for `project-mesh`. Step 3 is the one that cannot be undone.

- [x] Check the org name is free on GitHub. **betarcomms** (renamed 2026-07-29 from
      `Betar-Communication`, which was itself chosen over `betar-mesh`).
- [x] Create the organisation.
- [x] Transfer `project-mesh` into it. Done 2026-07-29 via the GitHub API
      (`POST /repos/konkomaji/project-mesh/transfer`), confirmed live at
      `betarcomms/project-mesh`; old `konkomaji/project-mesh` URL redirects, local `origin`
      remote updated to match.
      **Note: it went in already public, not private-first as originally planned here.**
      `konkomaji/project-mesh` was already public before this doc's private-first,
      scan-then-public order could apply — flagged in the prior session, unchanged by the
      transfer itself. The secret scan below still stands as the check that matters.
      **User instruction 2026-07-29: `project-mesh` stays as-is otherwise** — no
      `SECURITY.md`/`CONTRIBUTING.md`/`CODE_OF_CONDUCT.md`/`GOVERNANCE.md` link added as
      part of this transfer. Those checklist lines below are left open, not done.
- [x] **Scan git history for secrets before going public.** Done 2026-07-27 with
      `gitleaks detect --no-git=false` over full history (69 commits). 3 findings, all
      false positives: the crate name `libcrux-sha3` in two `Cargo.lock` files, and the
      SharedPreferences key constant name `joined_passphrases_wrapped_b64` in
      `ChannelMessaging.kt`, none of them an actual secret value. Filename check for
      `local.properties`, `.jks`, `.keystore`, and `android/app/src/main/jniLibs` binaries
      also clean, nothing committed that should not be. Re-run this scan for `betar` and
      `betarchat` once code actually lands in them — today they're empty, nothing to scan.
- [ ] Add `SECURITY.md`. How to report a vulnerability, and a clear line that no
      independent audit has happened yet. Researchers look here first.
- [ ] Add `CONTRIBUTING.md`, including the framing rules in Part 2 so a contributor does
      not undo the positioning in a pull request description.
- [ ] Add `CODE_OF_CONDUCT.md`.
- [x] Set the repo description and topics: `mesh-networking`, `ble`, `offline-first`,
      `disaster-response`, `rust`, `android`. Already set on the GitHub repo before this
      session (description: "Betar: offline mesh messaging and safety infrastructure...";
      topics include all six listed plus `end-to-end-encryption`, `jetpack-compose`,
      `kotlin`, `wifi-direct`) — confirmed via the API response from the transfer above,
      not something this session did.
- [ ] Link `GOVERNANCE.md` from the README.
- [ ] Make the repo public. (Already public, see the note above. Re-verify the secret
      scan is clean rather than skipping this step.)

**Decided 2026-07-29, documentation policy for `betar` and `betarchat` specifically:**
these two repos do **not** get `project-mesh`'s style of detailed internal working docs
(no `IMPLEMENTATION-STATUS.md`-style gap tracker, no `BETAR-TRANSITION.md`-style running
log carried over). Just the normal set a human-written GitHub repo has — README,
LICENSE, CONTRIBUTING, that's it, written plainly. `project-mesh` itself is unaffected
by this and keeps its existing docs as-is per the instruction above; this is scoped to
what gets written when `betar`/`betarchat` actually get code.

## Part 4. Documentation rewrite

Two separate jobs happening in the same pass: strip the framing that has to go, and fix
the voice.

### 4.1 Scope

A search for government, shutdown, censorship, circumvention, legal instrument and
political terms across all markdown returned **218 matches in 19 files**.

| File | Matches |
|---|---|
| `docs/PROGRESS.md` | 60 |
| `WHITEPAPER.md` | 36 |
| `docs/LEGAL.md` | 35 |
| `docs/RESEARCH-FINDINGS.md` | 26 |
| `docs/IMPLEMENTATION-STATUS.md` | 17 |
| `docs/REFERENCES.md` | 16 |
| `docs/CRYPTOGRAPHY.md` | 5 |
| `README.md` | 4 |
| `docs/TRANSPORT.md` | 3 |
| `docs/THREAT-MODEL.md` | 3 |
| `docs/GOVERNANCE.md` | 3 |
| `docs/DISTRIBUTION.md` | 2 |
| `docs/REPRODUCIBLE-BUILD.md` | 2 |
| `docs/ROUTING-PROTOCOL.md` | 1 |
| `docs/ROADMAP.md` | 1 |
| `docs/HARDWARE-LORA.md` | 1 |
| `docs/ARCHITECTURE.md` | 1 |
| `docs/LOCALIZATION-UX.md` | 1 |
| `core/fuzz/README.md` | 1 |

That number is an upper bound, not a work list. The search included the words "state" and
"political", so it catches things like `InvalidState` and "stated plainly" that have
nothing to do with framing. Every hit needs looking at rather than replacing blind.

### 4.2 What to change

- [ ] Remove all government, shutdown and political framing. Replace with storms, damage,
      distance, dead zones and outages.
- [ ] Remove every em dash across all documentation.
- [ ] Rewrite the voice. Plain sentences, no repeated honesty tics.
- [ ] Rename to Betar where it means the app. Keep Project Mesh where it means the
      protocol or the Rust core.
- [ ] Update every repository URL to `https://github.com/betarcomms/project-mesh`.
- [ ] Remove any wording that names or implies a single developer.
- [ ] Replace the tricolor logo reference in `README.md` with `docs/assets/betar-logo.svg`.

### 4.3 Two calls that need making first

**`docs/LEGAL.md`.** 130 lines, and roughly 80 percent of it is the government framing
doctrine that is being removed. Two options:

- **Slim it.** Keep LoRa spectrum compliance, the licence, and the no warranty position.
  Cut everything else. Rename to `docs/COMPLIANCE.md`.
- **Delete it.** Move the licence line into `GOVERNANCE.md` and the IN865 band rules into
  `HARDWARE-LORA.md`, then remove the file.

**`docs/PROGRESS.md`.** Its own header says it is append only, and it is the honest paper
trail of how the project got here. Rewriting a log after the fact is a different act from
editing a specification. Options:

- Leave the existing entries alone and apply the new rules only to entries from here on.
- Rewrite it like everything else, and accept that the log no longer reflects what was
  actually written at the time.

Recommendation is to leave it alone, note the change of rules in a dated entry, and move
on. It is a log, and 60 of the 218 hits are in it.

## Part 5. Code and app changes

Package id decided and renamed in code. Icons are not done.

- [x] **Decide the Android package id.** Was `india.projectmesh.app`. This was the
      single irreversible choice on this page. F-Droid and Play both treat the package id
      as the app's permanent identity, and changing it later means a new app with zero
      installs.

      **First decided `in.betar.app` / `in.betar.chat`, then rejected: `in` is a Kotlin
      hard keyword.** Confirmed by direct test-compile against this project's own
      toolchain, not just reasoned about: `package in.betar.scratch` fails
      (`Syntax error: Package name must be a '.'-separated identifier list.`).
      `` package `in`.betar.scratch `` (backtick-escaped) does compile, but every one of
      this app's ~90 Kotlin files would carry that escape permanently on every
      package/import line touching the root — rejected as a standing tax on the whole
      codebase for a package id.

      **Decided instead: `app.betar.comm` for Betar, `app.betar.chat` for Betar Chat.**
      Base reverses to `betar.app`, a domain worth actually holding (same ownership
      caveat that ruled out `org.betar.*` without `betar.org` held — noted, not yet
      verified as registered). `.comm` on the full app for "communication" (matches the
      GitHub org name, betarcomms); `.chat` on the chat-only variant for what it
      actually is. (Briefly renamed to bare `app.betar` for the full app mid-session, then
      corrected to `app.betar.comm` — the code below reflects the final id only.)

      **Checked 2026-07-29 (before this exact `app.betar.comm` / `app.betar.chat` pair was
      settled, so re-check before any store submission):** `in.betar.app` / `in.betar.chat`
      didn't collide on Play or F-Droid at the time. The current ids have not been
      checked.
- [x] Change the app label to Betar in `strings.xml`. Already true going into this
      session (`values/strings.xml` and `values-bn/strings.xml` both already said
      `Betar` / `বেতার`) — not something this rename had to do.
- [x] Rename the Kotlin package to match the new package id. Done 2026-07-29: moved
      `android/app/src/main/java/india/projectmesh/app/**` to
      `android/app/src/main/java/app/betar/comm/**`, rewrote every package/import/FQN
      reference, updated `namespace`/`applicationId` in `build.gradle.kts` to
      `app.betar.comm`, renamed the `Theme.ProjectMesh` style to `Theme.Betar`, renamed
      `metadata/india.projectmesh.app.yml` to `metadata/app.betar.comm.yml`. Verified with
      `./gradlew :app:compileDebugKotlin` — BUILD SUCCESSFUL. Not run on a device this
      session.
- [ ] Build the adaptive launcher icon from the locked mark. **Build it inverted**:
      background layer filled `#4BA3E0` edge to edge, foreground layer drawing the two
      wire bars in `#EEF4F9`. An adaptive icon cannot have a real hole because the
      foreground sits on the background rather than on the wallpaper.
- [ ] Add the monochrome icon layer for Android 13 themed icons.
- [ ] Replace the old tricolor drawables once the new ones are in.
- [x] Update `metadata/india.projectmesh.app.yml` to the new package id and app name.
      Renamed to `metadata/app.betar.comm.yml`, `AutoName` changed to `Betar`. `SourceCode`/
      `Repo`/`IssueTracker` URLs deliberately left pointing at the current
      `konkomaji/project-mesh` mono-repo — Part 3's repo split hasn't happened, updating
      those now would be premature.

## Part 6. Design

- [x] Logo designed and delivered, `docs/assets/`. **This logo is Betar's only.**
- [ ] **Decided 2026-07-29: Betar Chat needs its own separate logo, not a reuse or minor
      variant of Betar's.** Not designed yet. Betar Chat is otherwise unbriefed —
      no design brief, no concept round — this is a real gap, not just an asset to
      produce off the existing brief.
- [x] Design brief written, `docs/DESIGN-BRIEF.md`. Covers Betar only.
- [ ] Run the brief through a designer or a design tool. Get the design system agreed
      before the full screen set.
- [ ] Missing assets, once the system exists: wordmark lockup (needs a typeface chosen
      first, guessing letterforms is how logos end up looking wrong), feature graphic,
      store screenshots, the category pictogram set. All Betar-specific; Betar Chat needs
      its own equivalent set once its own logo/brief exist.

### 6.1 Websites

Each of the three repos now has a live GitHub Pages site (`docs/` folder on `main`),
per-repo, done 2026-07-29:

- [x] **Betar** — `https://betarcomms.github.io/betar/`. Moved as-is from `project-mesh`
      (where it was built during the mono-repo era): home, about, safety, privacy,
      documents. Material 3 Expressive motion/shape language transcribed from the app's
      own design system, Betar's brand palette (`#4BA3E0` etc.), SEO/AEO metadata
      (JSON-LD `SoftwareApplication` + `FAQPage`, sitemap, `llms.txt`, OpenGraph).
- [x] **Project Mesh** — `https://betarcomms.github.io/project-mesh/`. Rewritten from
      scratch, framed as the research/protocol repo rather than the app: transport/
      routing/cryptography overview, whitepaper and threat-model links, FAQ, matching
      SEO/AEO metadata. **Deliberately not using Betar's brand blue or the old tricolor
      mesh glyph** (`docs/assets/logo.svg` — predates the Betar rebrand, is the exact
      file the README checklist below already flags for removal, reusing it here would
      have contradicted that). Ships with a clean wordmark and a distinct neutral
      graphite palette instead — same motion/shape system as Betar's site for family
      resemblance, but **Project Mesh does not have its own locked visual mark**, this
      is a real open gap, not a finished identity.
- [x] **Betar Chat** — `https://betarcomms.github.io/betarchat/`. Single honest
      placeholder page, neutral/unbranded styling, states plainly that there's no app
      code and no logo yet rather than implying a finished product. Not a "full
      redesign" in any real sense — there is nothing to redesign until the logo (see
      above) and a design brief exist.

**Open gap, stated plainly:** Project Mesh's own visual identity (an actual mark, not
just a wordmark) doesn't exist. The only asset that ever existed for this name
(`docs/assets/logo.svg`) is the pre-rebrand tricolor glyph already being retired.
Designing one is separate work from Betar Chat's logo gap above, not done here.

## Part 7. Distribution

- [ ] Configure real release signing. Releases are debug signed today.
- [ ] Validate the F-Droid metadata against F-Droid's own linter.
- [ ] Submit to F-Droid. Not attempted yet.
- [ ] Test cross machine reproducibility. Only same machine has been verified.
- [ ] Test full APK reproducibility. Only the Rust `.so` has been checked.
- [ ] Write the store listing copy, following the framing rules in Part 2. First line is
      the promise, second line is remote areas and cyclones, messaging comes third.

## Part 8. Still true, still not done

Carried over from `IMPLEMENTATION-STATUS.md` because these matter for any release
decision:

- No two device or physical hardware test. The dev machine cannot run two emulators.
- No independent security review. This is a hard gate before any general release.
- The fuzzing harness cannot run on Windows because of an MSVC linker conflict. It needs
  a Linux machine or CI.
- No CI exists at all.
- English only. The string framework works, the translations do not exist.
- No QR code trust establishment, no leave group, no leave channel, no member removal.
- MLS commit distribution to other members is still a manual hex paste.

## Part 9. Suggested order

1. Settle the package id and the two calls in 4.3. Everything downstream depends on them.
2. Create the org, transfer the repo, scan history, add the repo files, go public.
3. Run the documentation rewrite in one pass, so the voice stays consistent.
4. Do the code rename and the launcher icon together, in one commit.
5. Run the design brief.
6. Distribution last.
