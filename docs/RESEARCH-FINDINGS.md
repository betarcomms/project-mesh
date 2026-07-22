# Research Findings and Verification Report

**Deep multi-source research + adversarial verification for Project MESH.**
Method: fan-out web search across 7 angles → source fetch → claim extraction (114 claims) →
3-vote adversarial verification (74 of 75 verdicts confirmed against primary sources; 1 minor
wording nuance refuted). Confidence levels and sources are noted per finding. Dates reflect
sources current to early–mid 2026.

> This report is the evidence base for the corrections applied across the documentation set. It
> supersedes any conflicting statement elsewhere in the repo; affected files have been updated to
> match.

---

## 0. Executive summary — what changed

| # | Finding | Action taken |
|---|---|---|
| 1 | India's licence-free band is **865–868 MHz** (not 865–867), per GSR 853(E), 10 Dec 2021 | Corrected `HARDWARE-LORA.md`, `TRANSPORT.md`, `WHITEPAPER.md` |
| 2 | **Android CAN discover a backgrounded iOS BLE peripheral** by parsing Apple's overflow bitmask | Corrected `TRANSPORT.md` (was overstated as impossible) |
| 3 | 2017 shutdown rules **replaced by Telecommunications (Temporary Suspension of Services) Rules, 2024** under the Telecom Act 2023 | Rewrote `LEGAL.md` |
| 4 | **MLS (RFC 9420)** is the right choice for groups; Double Ratchet is 1:1 only | Updated `CRYPTOGRAPHY.md`, `WHITEPAPER.md` |
| 5 | **Post-quantum** (PQXDH / ML-KEM) is now standard for "most secure" | Added PQ layer to `CRYPTOGRAPHY.md` |
| 6 | **No mesh is "untraceable"** — BLE physical-layer fingerprinting defeats MAC randomization | Hardened `THREAT-MODEL.md`; removed any absolute claim |
| 7 | **Reticulum** is a strong reference but risky to hard-depend on (no formal spec; founder stepped back Dec 2025) | Added §1 guidance; kept custom protocol, interop where useful |

---

## 1. Global state of the art (2026)

### Reticulum (RNS + LXMF) — the most important reference
- **Confirmed (high):** a cryptography-based networking stack running across **LoRa, packet
  radio, WiFi/Ethernet, serial, I2P** — heterogeneous by design, matching MESH's multi-transport
  goal. Crypto: **X25519** ECDH, **Ed25519** signatures, **AES-256-CBC** (PKCS7), **HMAC-SHA256**,
  **HKDF**. Works on very low-bandwidth links (>~5 bit/s, 500-byte MTU). Provides **initiator
  anonymity** — packets omit the source address, so the sender reveals no identity. Encrypted
  link setup ≈ 3 packets / 297 bytes. Coordination-less globally-unique addressing.
- **Risks (high confidence):**
  - **No formal protocol specification.** As of FOSDEM 2026 a *draft* spec + conformance/test
    vectors were only just being initiated; historically "the reference implementation is the
    authoritative spec." Hard to build an independent, auditable implementation against.
  - **Bus factor.** Founder **Mark Qvist stepped back from all public coordination (Dec 2025)**;
    governance is transitioning to a nascent community model. Real maintenance-continuity risk.
  - **Bluetooth is via RNode/serial**, not a native phone-to-phone BLE mesh transport.
- **Recommendation:** treat Reticulum as a **primary design reference and interop target**
  (especially its initiator-anonymity wire design and LoRa transport), but **do not adopt it as
  MESH's sole protocol foundation** given the spec + governance risk. Watch the Rust
  reimplementation (`reticulum-rs`) — if it matures, revisit.

### Security lessons from broken apps (all high confidence)
- **Bridgefy — broken twice.** 2020 (Albrecht et al., CT-RSA 2021): no authenticity
  (impersonation), no effective confidentiality (broken RSA/PKCS#1), user tracking + social-graph
  reconstruction, single crafted message could DoS the whole mesh. 2022 ("Breaking Bridgefy,
  again"): **even after migrating to the Signal protocol**, a TOCTOU bug broke confidentiality,
  compression-before-encryption leaked plaintext (CRIME/BREACH-style), TOFU with no key
  verification allowed MITM, and **social-graph reconstruction persisted**. **Lesson: using
  libsignal is not enough if integrated wrong, and payload encryption does not hide metadata.**
- **Bitchat (2025).** Marketed as secure/E2EE but **no external review at launch**; researcher
  Alex Radocea found an identity-authentication flaw enabling contact impersonation, plus a
  possible buffer overflow and questionable forward secrecy. **Lesson: audit before claims.**
- **Berty (Oct 2025).** Wesh protocol, BLE, server-less — but Android app was **unavailable
  pending security updates**, docs have gaps. **Lesson: maintenance and clarity matter.**
- **Session (Dec 2025).** V2 protocol **reintroduces forward secrecy** (removed in 2021), adopts
  **ML-KEM** post-quantum, keeps **onion routing**. Direction-of-travel signal for "most secure."

---

## 2. Technology stack — confirmed decisions

### Language: Rust (confirmed)
- **Memory-safety demarcation (high):** in Rust, misuse of a non-`unsafe` API into a memory bug
  is by definition a library bug, not the caller's — a clear safety boundary C/C++ lack. Raw CVE
  comparisons mislead because ecosystems assign blame differently.
- **Bindings (high):** **UniFFI** auto-generates Kotlin (Android) and Swift (iOS) bindings from
  one interface. Mozilla uses **JNA (not JNI)** deliberately so the *same* Rust FFI is shared
  across platforms; hand-written FFI has real pitfalls (JNA bool width bug) — hence
  auto-generation. Fallback: flatten to `extern "C"` in one library and call via JNA.
- **Alternative worth noting:** **Crux** (Rust) shares a side-effect-free core across iOS/Android/
  web, compiling to native + WASM; pre-1.0 but production-used. Optional architecture reference.
- **Verdict: Rust core + UniFFI stands.** No stronger option for a security-critical shared core.

### Crypto protocols — updated
- **1:1 messaging:** **Double Ratchet** (Perrin/Marlinspike, Rev 4, Nov 2025) — FS + PCS,
  Curve25519, HKDF-SHA256, AES-256. It is **fundamentally two-party.**
- **Groups:** **switch to MLS (RFC 9420, Jul 2023).** MLS gives async group key establishment with
  FS **and** PCS and **logarithmic** key-update cost via **TreeKEM** — vs Double-Ratchet/sender-key
  group schemes whose update cost grows **~O(N²)**. MESH's original "seal one copy per member"
  (O(N)) is acceptable only for tiny groups; **MLS is the correct choice for anything larger.**
- **Post-quantum:** add **PQXDH** (Signal's PQ extension of X3DH; **ML-KEM/Kyber-1024** in
  production) to the handshake. It stops **"harvest-now, decrypt-later"** passive quantum attacks
  but **not** an active quantum attacker (honest limit). MLS cipher suites are HPKE-based and have
  PQ paths. **Recommendation: hybrid classical + PQ handshake from day one for "most secure."**
  - *Verified nuance:* PQXDH prescribes a KEM *property* (IND-CCA PQ security); Kyber-1024 is the
    named example and Signal's production choice, though the spec words it as "e.g.," not a formal
    "recommend."
- **Handshake framework:** Noise remains fine for link setup; combine with the PQ KEM.

### Mapping
- **MapLibre + OpenStreetMap + offline vector tiles (MBTiles/PMTiles)** — unchanged; confirmed as
  the de-Googled offline mapping stack.

---

## 3. iOS background BLE — verified, with a correction

Primary source: Apple Core Bluetooth Background Processing Guide, corroborated by independent
reverse-engineering (David G. Young, "Hacking the Overflow Area").

- **Confirmed (high):** a backgrounded iOS app's service UUIDs are moved to a special **"overflow"
  area** (Apple manufacturer packet, code `0x004C`, type `0x01`) encoded as a **128-bit hashed
  bitmask** — each UUID sets one bit. Discoverable **only by a device explicitly scanning for that
  specific UUID** (a generic scan can't find it). Background local name is dropped; background
  advertise/scan is throttled and duplicate-coalesced. Since **iOS 14**, an app **cannot change
  which services it advertises while backgrounded**.
- **CORRECTION (high):** **Android *can* discover a backgrounded iOS peripheral** by parsing the
  Apple overflow manufacturer packet and testing the relevant bit in the bitmask — cross-platform
  background discovery **is possible** if Android knows the specific UUID to look for. My earlier
  docs overstated this as impossible.
- **Caveat (high):** overflow advertisements are **only transmitted while the sending device's
  screen is illuminated** (screen on; need not be unlocked/foregrounded). So background iOS relay
  is **degraded and screen-gated**, not zero. The "keep the app in foreground / screen-on relay
  mode" UX still stands, but the nuance is: Android bridging to a screen-on-but-backgrounded iOS
  device works if it scans for the known UUID.

**Net:** iOS background relay is **limited and screen-dependent, not strictly impossible**; the
honest UX guidance is unchanged, but the technical claim is now precise.

---

## 4. India LoRa band — verified, with a correction

Primary source: **Gazette of India G.S.R. 853(E), 10 December 2021**, "Use of Low Power Equipment
in the Frequency Band 865–868 MHz for Short Range Devices (Exemption from Licence) Rules, 2021"
(Ministry of Communications, WPC Wing), verified by extracting the government PDF text.

- **CORRECTION (high):** the licence-free band is **865–868 MHz**, delicensed by GSR 853(E),
  **superseding** the 2005 865–867 MHz RFID rules. (My docs said 865–867.)
- **Non-Specific Short Range Devices (verbatim):** **25 mW e.r.p.**, **1% duty cycle**, **FHSS**,
  **max occupied bandwidth 50 kHz for 58 or more hop channels** (duty cycle applies to the whole
  transmission, not per hop). Standard EN 300 220 referenced.
- **Tracking/Tracing & Data Acquisition Devices:** up to **500 mW e.r.p.** with **Adaptive Power
  Control required**, duty cycle **10%** for network access points / **2.5%** otherwise, **200 kHz**
  bandwidth.
- **Conditions:** no licence needed for compliant devices, but on a **non-interference,
  non-protected, shared** basis, and equipment must be **type-approved** by the Central
  Government. Legal basis: Indian Telegraph Act 1885 + Indian Wireless Telegraphy Act 1933.

**Design impact:** LoRa firmware must target **IN865 within 865–868 MHz**, and the routing engine
must enforce the **1% duty cycle** (25 mW class) — a hard throughput ceiling that reinforces
"small high-priority messages only over LoRa." Higher-power 500 mW operation needs APC and is a
different device class.

---

## 5. Indian law — verified, substantially updated

Sources: SFLC.in, Internet Freedom Foundation, SCC Online, official gazette notifications;
corroborated across multiple independent outlets.

- **Shutdown framework replaced (high).** The **Telecommunications (Temporary Suspension of
  Services) Rules, 2024** were notified and came into force **22 November 2024** under the
  **Telecommunications Act, 2023**, **superseding** the 2017 Temporary Suspension of Telecom
  Services Rules (and the Telegraph Act basis). Changes: a single order is capped at **15 days**
  (Rule 3(2)(b)(iii)); orders must specify the **geographical region**; but the **Review Committee
  (Rule 6) is executive-only** (Home + Law officials), no judicial/public oversight. In practice,
  shutdown behaviour is "new language, same shutdowns" (e.g. Manipur, J&K).
- **Telecom Act 2023 (high).** Partially in force from **26 June 2024**. **Section 20** empowers the
  Union to take possession of / suspend / intercept telecom services and messages on public
  emergency/safety grounds. **Section 29** imposes a duty on users **not to furnish false
  identity information** to avail services — **if applied to internet/OTT services, this threatens
  anonymous communication.** Whether **OTT / E2EE services fall under the Act is legally
  ambiguous**: a Minister's Dec 2023 verbal exclusion is **not legally binding** (the text doesn't
  exclude OTT), and OTT licensing was still under inter-ministerial consultation in 2024.
- **IT Rules 2021 traceability (high).** **Rule 4(2)** requires "significant social media
  intermediaries" to enable identification of the **first originator** of a message — challenged
  by **WhatsApp/Facebook (Delhi HC)** and a **FOSS developer, Praveen Arimbrathodiyil (Kerala
  HC)** as breaking E2EE and violating the Article 21 privacy right; litigation ongoing across
  multiple courts.
- **Anuradha Bhasin v. Union of India (2020) 3 SCC 637 (high).** Shutdowns must be **legal,
  necessary, proportionate, time-bound, reasoned, published, and reviewable**; **indefinite
  shutdowns are unconstitutional.** Still the governing precedent.

**Implication for MESH (reasoned, not adjudicated):** a **server-less, operator-less,
account-less** app has **no first-originator to produce** and does not run as a "significant
social media intermediary" in the ordinary case — so Rule 4(2) has nothing to attach to. **But**
the **Telecom Act 2023 §29 anti-anonymity duty** and the **unresolved OTT/E2EE scope** are new
risk vectors that did not exist when the older analysis was written. **Counsel review (SFLC.in)
is now more important, not less.** These are updated in `LEGAL.md`.

---

## 6. Traceability & hackability — the honest reality

The user's goal is "not traceable, not hackable." The verified evidence forces an honest
correction: **that is not achievable, and no credible project claims it.** What is achievable is
**hard-to-trace, hard-to-hack, and no central point to seize.**

- **BLE physical-layer fingerprinting (IEEE S&P 2022, high).** Hardware imperfections —
  **Carrier Frequency Offset** and **I/Q offset** — let an attacker fingerprint a BLE device at
  the physical layer, **bypassing MAC-address randomization**. A **~$150 software-defined radio**
  suffices. Modern phones advertise constantly (an iPhone emits **~872 BLE advertisements/minute**
  when idle), enabling passive tracking. **Reliability is limited** (thermal drift, chipset
  variation, TX-power, receiver quality), so it is **feasible but not universally reliable** — an
  honest "sometimes trackable," not "always" and not "never."
- **Metadata is the hard part (high).** BLE mesh apps have been broken primarily by **failing to
  hide metadata**, which is inherent to Bluetooth's broadcast nature. Strong anonymity needs a
  **purpose-built** protocol; **generic mesh does not provide anonymity**, and metadata-private
  broadcast-mesh designs **scale only to moderate size.**
- **Other attack surface:** Sybil/eclipse, jamming (2.4 GHz BLE/WiFi is jammable; sub-GHz LoRa is
  a different band, a mitigation not a cure), timing/traffic correlation, and endpoint compromise
  (if the device is owned, plaintext is exposed).

**Mandated doc change:** every "untraceable / unhackable / anonymous" absolute is removed and
replaced with bounded, honest language and an explicit statement that a determined local RF
adversary may fingerprint/track devices despite MAC randomization. This honesty is itself a
security and legal asset (§0, `THREAT-MODEL.md`, `LEGAL.md`).

---

## 7. Loopholes & critical gaps in the original design

Prioritized, with the fix applied or recommended:

1. **[Fixed] Group crypto scaling.** Original O(N) per-member sealing → **adopt MLS (RFC 9420)**
   for groups; keep Double Ratchet for 1:1.
2. **[Fixed] No post-quantum.** Add **PQXDH / ML-KEM hybrid** handshake.
3. **[Fixed] Overstated anonymity.** "Untraceable" claims removed; **BLE physical-layer
   fingerprinting** added to the threat model as a real, only-partly-mitigable risk.
4. **[Fixed] Wrong LoRa band + missing duty cycle.** 865–868 MHz, 25 mW e.r.p., **1% duty cycle**
   enforced in the routing engine.
5. **[Fixed] iOS/Android background discovery overstated as impossible.** Corrected; screen-gated
   nuance documented.
6. **[Fixed] Stale shutdown law.** 2017 rules → **2024 Suspension Rules under Telecom Act 2023**;
   added **§29 anti-anonymity** and **OTT/E2EE scope** risks.
7. **[Open] TOFU without verification is a known break vector** (Bridgefy 2022). MESH already
   mandates in-person QR/safety-string verification — keep it **mandatory and prominent**, never
   silent TOFU.
8. **[Open] "libsignal/Noise is enough" fallacy.** Integration bugs (TOCTOU, compress-then-encrypt)
   broke Bridgefy despite Signal. Mandate: **no compression before encryption**, constant-time
   handling, and an **external audit gate** before any security claim ships.
9. **[Open] Reticulum dependency risk.** Do not hard-depend; interop/borrow only, given no formal
   spec + founder withdrawal.
10. **[Open] Metadata-private routing scaling.** Onion/anonymous mesh routing scales only to
    moderate size — document as a known limit; do not promise network-wide anonymity.

---

## 8. Prioritized recommendations

**P0 (do before any code claims security):**
- Rust core + UniFFI (confirmed).
- 1:1 = Double Ratchet; **groups = MLS (RFC 9420)**.
- **Hybrid PQ handshake (PQXDH / ML-KEM)**.
- **Mandatory** in-person key verification (no silent TOFU).
- **No compression-before-encryption**; fixed-size padded envelopes.
- **External security audit is a hard release gate.**

**P1 (correctness / compliance):**
- LoRa firmware to **865–868 MHz IN865**, enforce **1% duty cycle** (25 mW class) in routing.
- Legal docs updated to **2024 Suspension Rules + Telecom Act 2023**; obtain **SFLC.in review**,
  specifically on §29 anti-anonymity and OTT/E2EE scope.
- Remove all "untraceable/unhackable" language; ship honest bounded claims.

**P2 (strategy / longevity):**
- Track **`reticulum-rs`** and the Reticulum spec effort; consider interop later.
- Watch **Session V2** (ML-KEM + PFS + onion) as a design bellwether.
- Keep the design's honesty (documented limits) as a first-class feature.

---

## 9. Source quality note

Findings above are drawn from **primary sources wherever possible** — Apple developer
documentation, the Gazette of India GSR 853(E) PDF, Signal/IETF specifications (PQXDH, Double
Ratchet Rev 4, RFC 9420), peer-reviewed security papers (CT-RSA 2021, USENIX 2022, IEEE S&P
2022), and official/authoritative legal analyses (SFLC.in, IFF, SCC Online, gazette
notifications) — and cross-checked by 3-vote adversarial verification. The full bibliography is
in `REFERENCES.md`. A handful of forward-looking items (Session V2, Reticulum governance) are
current as of late-2025/early-2026 and should be re-checked before publication.
