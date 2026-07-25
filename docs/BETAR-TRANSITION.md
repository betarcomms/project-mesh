# Betar Transition: everything still to do

Working document. The project is being renamed, repositioned and made public, and this
tracks all of it in one place so nothing gets lost between sessions.

Status of this file: nothing in Part 3 onwards has been done yet. Parts 1 and 2 record
what has already been decided, so future work does not relitigate settled questions.

---

## Part 1. Decided

| Thing | Decision |
|---|---|
| App name | **Betar** (বেতার), Bengali for wireless, literally *be-* (without) *tar* (wire) |
| Protocol name | **Project Mesh** stays as the name of the protocol and the Rust core |
| GitHub org | **betar-mesh** (fallbacks: `betarnet`, `betar-project`, `betar-community`) |
| Main repo name | Stays **project-mesh**, moved under the org |
| Repo URL | `https://github.com/betar-mesh/project-mesh` |
| Repo visibility | Going public |
| Logo | Done. "The wire that is not there", files in `docs/assets/` |
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

Do these in order. Step 3 is the one that cannot be undone.

- [ ] Check `betar-mesh` is free on GitHub. Use a fallback if not.
- [ ] Create the organisation.
- [ ] Transfer `project-mesh` into it **while still private**. History, issues and stars
      survive, and GitHub redirects the old URL.
- [ ] **Scan git history for secrets before going public.** Once public, history is
      permanent and deleting a file does not remove it. Run `gitleaks detect
      --no-git=false`, and check by hand for `local.properties`, any `.jks` or
      `.keystore`, and anything under `android/app/src/main/jniLibs` that should not be
      committed.
- [ ] Add `SECURITY.md`. How to report a vulnerability, and a clear line that no
      independent audit has happened yet. Researchers look here first.
- [ ] Add `CONTRIBUTING.md`, including the framing rules in Part 2 so a contributor does
      not undo the positioning in a pull request description.
- [ ] Add `CODE_OF_CONDUCT.md`.
- [ ] Set the repo description and topics: `mesh-networking`, `ble`, `offline-first`,
      `disaster-response`, `rust`, `android`.
- [ ] Link `GOVERNANCE.md` from the README.
- [ ] Make the repo public.

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
- [ ] Update every repository URL to `https://github.com/betar-mesh/project-mesh`.
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

None of this has been started. It all waits until the rename is settled.

- [ ] **Decide the Android package id.** Currently `india.projectmesh.app`. This is the
      single irreversible choice on this page. F-Droid and Play both treat the package id
      as the app's permanent identity, and changing it later means a new app with zero
      installs. Candidates: `mesh.betar.app`, `in.betar.app`. Do not use `org.betar.*`
      unless the `betar.org` domain is actually held.
- [ ] Change the app label to Betar in `strings.xml`.
- [ ] Rename the Kotlin package to match the new package id.
- [ ] Build the adaptive launcher icon from the locked mark. **Build it inverted**:
      background layer filled `#4BA3E0` edge to edge, foreground layer drawing the two
      wire bars in `#EEF4F9`. An adaptive icon cannot have a real hole because the
      foreground sits on the background rather than on the wallpaper.
- [ ] Add the monochrome icon layer for Android 13 themed icons.
- [ ] Replace the old tricolor drawables once the new ones are in.
- [ ] Update `metadata/india.projectmesh.app.yml` to the new package id and app name.

## Part 6. Design

- [x] Logo designed and delivered, `docs/assets/`.
- [x] Design brief written, `docs/DESIGN-BRIEF.md`.
- [ ] Run the brief through a designer or a design tool. Get the design system agreed
      before the full screen set.
- [ ] Missing assets, once the system exists: wordmark lockup (needs a typeface chosen
      first, guessing letterforms is how logos end up looking wrong), feature graphic,
      store screenshots, the category pictogram set.

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
