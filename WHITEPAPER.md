# Project MESH: A Decentralized Off-Grid Communication and Civic-Resilience Network for India

**A technical white paper**

Stewardship: Konko Maji (research + open source)
Status: Design / pre-alpha — v0.1 draft
Licence: CC BY-SA 4.0

---

## Abstract

Communication systems in India fail in three recurring situations: in regions with weak or
absent cellular and fibre coverage (Ladakh, the North-East, the Sundarbans, Himalayan and
tribal belts); during natural disasters that physically destroy telecommunication
infrastructure (floods, cyclones, earthquakes); and during administrative shutdowns of
telecommunication services. All three failures share a single cause — dependence on
centralized, infrastructure-bound networks. This paper presents **Project MESH**, a
decentralized, server-less, end-to-end encrypted communication and civic-coordination platform
designed for the Indian context. Project MESH forms an ad-hoc **delay-tolerant mesh network**
directly between users' phones over Bluetooth Low Energy and Wi-Fi, and, in a later phase,
across a backbone of low-cost long-range **LoRa** radio nodes operating in India's licence-free
865–868 MHz band. It requires no cell tower, no internet service provider, no central server,
and no personal identifier (phone number or email). We describe the system's design goals,
layered architecture, transport strategy, store-carry-forward routing protocol, cryptographic
design, application-level civic features (emergency SOS, disaster bulletins, offline maps, a
community resource board, and messaging), localization and accessibility strategy for a
low-literacy, low-end-device, multilingual user base, distribution as a fully de-Googled
application, open-source governance model, and legal positioning under Indian law. We are
explicit throughout about the system's fundamental limitations — most importantly the severe
constraints Apple's iOS places on background Bluetooth operation, and the physical necessity of
dedicated radio hardware to cover rural distances — and about the security properties the
system deliberately does and does not provide.

---

## Table of contents

1. Introduction
2. Background and related work
3. Design goals and non-goals
4. Threat model (summary)
5. System architecture
6. Transport layer
7. Routing protocol
8. Cryptographic design
9. Application features
10. Localization, accessibility, and user experience
11. Distribution and the de-Googled requirement
12. Governance, licensing, and sustainability
13. Legal positioning
14. Evaluation and comparison with prior art
15. Limitations and open problems
16. Roadmap
17. Conclusion
18. References

Detailed protocol-level specifications for several sections live in the `docs/` directory and
are cross-referenced from here.

---

## 1. Introduction

### 1.1 The problem

The reliability of everyday communication in India is not uniform. For hundreds of millions of
people, connectivity is contingent — on terrain, on weather, on infrastructure investment, and
on administrative decisions. Three distinct but related failure modes recur:

1. **Coverage gaps.** Cellular and fibre networks are commercially concentrated in dense and
   prosperous areas. Mountainous, forested, riverine, and remote regions — Ladakh, Arunachal
   Pradesh and the wider North-East, the Sundarbans delta, tribal districts across central
   India — have thin, intermittent, or no coverage. For these communities, being "offline" is
   the normal condition, not the exception.

2. **Disaster-induced collapse.** Natural disasters destroy the very infrastructure that
   emergency response depends on. Annual floods in Assam and Bihar submerge towers and cut
   power to base stations; cyclones in the Bay of Bengal (Amphan, Yaas, and others) flatten
   coastal networks; Himalayan earthquakes and landslides sever fibre. Communication fails at
   the precise moment it is most needed for rescue and relief.

3. **Administrative shutdowns.** Under the Temporary Suspension of Telecom Services (Public
   Emergency or Public Safety) Rules, 2017, authorities can order the suspension of telecom
   services. India has consistently ranked as the country with the highest number of
   such shutdowns globally (see §13 and References). The Supreme Court of India, in
   *Anuradha Bhasin v. Union of India* (2020), held that indefinite shutdowns are impermissible
   and that access to information via the internet is protected under the constitutional right
   to freedom of speech and expression, subject to proportionality — establishing that
   resilient communication is a legitimate civic interest.

These three failure modes look different politically but are **identical technically**: a
communication system that assumes always-available centralized infrastructure stops working
when that infrastructure is absent, destroyed, or disabled.

### 1.2 The approach

Project MESH removes the assumption. Instead of routing messages through a tower and a server,
it treats every participating device as a node that stores, carries, and forwards messages on
behalf of the network. Two phones within radio range exchange messages directly; a message
addressed to someone out of range is carried by intermediate devices until it reaches its
destination or expires. No node is privileged; there is no centre to fail or to switch off.
This is the **delay-tolerant networking (DTN)** paradigm, adapted to consumer smartphones and,
later, to inexpensive long-range radio nodes.

### 1.3 Contributions

This paper contributes:

- a coherent **system design** for an India-specific civic-resilience mesh network spanning
  phone-to-phone short-range links and a long-range LoRa backbone;
- an honest, engineering-first treatment of the **hard constraints** (iOS background
  Bluetooth, rural distance, battery, low-end devices) that prior consumer mesh apps have
  tended to understate;
- a **cryptographic and routing design** that provides confidentiality, forward secrecy, and
  meaningful metadata protection without any server or registry;
- a **localization and accessibility** strategy for a genuinely Indian user base — many Indic
  languages, low literacy, and inexpensive hardware; and
- a **legal and governance** framework positioning the project as legitimate civic
  infrastructure under Indian law.

### 1.4 What this project is not

Project MESH is not a tool built to defeat any government, and it is not marketed as one. It is
disaster and rural-connectivity infrastructure whose resilience is uniform: it does not know or
care *why* the network is unavailable. This positioning is both sincere and load-bearing; it is
elaborated in §13 and `docs/LEGAL.md`, and all contributors are asked to honour it.

---

## 2. Background and related work

### 2.1 Delay-tolerant and mesh networking

The intellectual foundations are **delay-tolerant networking** (Fall, 2003) and
**store-carry-forward** routing for intermittently connected networks. Classic DTN routing
schemes relevant here include **Epidemic routing** (Vahdat & Becker, 2000), **Spray-and-Wait**
(Spyropoulos et al., 2005), and **PRoPHET** (Lindgren et al.). These trade bandwidth and
storage for delivery probability in networks with no stable end-to-end path — exactly the
regime Project MESH operates in.

### 2.2 Consumer mesh messaging apps

Several consumer applications have attempted phone-to-phone mesh messaging:

- **FireChat** (Open Garden) popularized the idea during protests in Hong Kong and elsewhere,
  and was used in India during past shutdowns. Its mesh support was discontinued around
  2018–2019. It was proprietary and not built for security.
- **Bridgefy** became widely known but was shown to be insecure: the *"Mesh Messaging in
  Large-Scale Protests: Breaking Bridgefy"* analysis (Albrecht et al., 2021) demonstrated
  practical attacks on confidentiality and integrity in its then-current form. It is treated
  here as a cautionary example, not a model.
- **Briar** is a mature, security-focused, open-source messenger for activists and journalists.
  It uses Bluetooth, Wi-Fi, and Tor, with strong cryptography. Its design is a primary positive
  reference, though it is Android-centric and chat-focused rather than a civic-resilience
  platform.
- **Bitchat** is a more recent open-source Bluetooth-mesh messenger.
- **Berty** is an open-source, decentralized messenger using BLE and internet transports.
- **ProtestChat** (ni5arga) is a recent open-source alpha built with React Native/Expo, using a
  BLE-mesh epidemic-relay model. It validated the demand but is Bluetooth-only, chat-only,
  built on a cross-platform runtime we deliberately avoid (§5), and — by its authors' own
  documentation — lacks background relaying, a completed ratchet, and any security audit. Its
  forward-secrecy and threat-model documents are useful references.

### 2.3 Long-range radio mesh

- **Meshtastic** is an open-source project that builds off-grid mesh networks over **LoRa**
  radio using inexpensive ESP32 boards, achieving kilometre-scale range. It supports the Indian
  **IN865** frequency region. Meshtastic is the primary positive reference for Project MESH's
  hardware backbone (Phase 3), though it is a generic communicator rather than an India-focused
  civic platform, and its phone integration is companion-device based.

### 2.4 Supporting technology

- **The Noise Protocol Framework** (Perrin) for authenticated key exchange.
- **The Signal Double Ratchet** (Marlinspike & Perrin) for forward-secret, self-healing session
  encryption.
- **Sphinx** (Danezis & Goldberg, 2009) and mix-network designs (Loopix; Nym) for
  metadata-protecting onion routing.
- **OpenStreetMap** and **MapLibre GL Native** with offline vector tiles (MBTiles/PMTiles) for
  fully offline, de-Googled mapping.
- **Rust** with **UniFFI** (Mozilla) for a single, memory-safe core shared across native
  Android and iOS front-ends.

### 2.5 Where Project MESH differs

No existing project simultaneously offers: (a) a security-first design, (b) a hybrid
phone-plus-LoRa transport tuned for Indian geography and the Indian LoRa band, (c) a
civic-resilience feature set beyond chat (SOS, disaster bulletins, offline maps, resource
sharing), (d) first-class localization for many Indic languages and low-literacy users on
low-end devices, and (e) a fully de-Googled distribution. Project MESH targets that
combination.

---

## 3. Design goals and non-goals

### 3.1 Goals

- **G1 — Infrastructure independence.** Core functions must work with zero internet, zero
  cellular service, and zero central server.
- **G2 — Decentralization.** No node is required; no registry, directory, or coordinator
  exists. The network degrades gracefully as nodes join or leave.
- **G3 — Security by default.** End-to-end encryption, forward secrecy, and no personal
  identifiers, with no configuration required from the user.
- **G4 — Metadata minimization.** Reduce what a passive observer or a relay can learn about who
  is talking to whom.
- **G5 — Civic utility.** Deliver real value in the *ordinary* rural and disaster case — not
  only in edge scenarios — through SOS, bulletins, maps, and resource coordination.
- **G6 — Accessibility.** Usable by low-literacy users, in many Indian languages, on
  inexpensive Android phones with limited RAM, storage, and battery.
- **G7 — De-Googled and libre.** No Google Play Services, no Firebase, no proprietary cloud
  dependency; distributable via F-Droid and direct download; open source.
- **G8 — Honesty.** Ship no claim we cannot substantiate; document every limitation.

### 3.2 Non-goals

- **N1 — Anonymity against a global passive adversary.** We provide meaningful metadata
  protection, not guarantees against an adversary who can observe the entire radio environment.
- **N2 — Real-time, high-bandwidth media.** This is a store-carry-forward text-and-small-payload
  network, not a video-calling platform.
- **N3 — Seamless iOS background operation.** Apple's platform constraints make this
  impossible; we design *around* it rather than pretending to solve it (§6).
- **N4 — Guaranteed delivery.** Delivery is best-effort and probabilistic, as is inherent to
  DTN.

---

## 4. Threat model (summary)

The full treatment is in [`docs/THREAT-MODEL.md`](docs/THREAT-MODEL.md). In brief:

**Assets:** message content; the social graph (who talks to whom); a user's location and
presence; a user's identity.

**Adversaries considered:**
- a **passive local eavesdropper** sniffing Bluetooth/Wi-Fi/LoRa in radio range;
- a **malicious participant** running a modified node to inject, drop, replay, or flood;
- a **local active attacker** attempting jamming or Sybil flooding;
- a **coercive physical adversary** who seizes a device.

**Explicitly out of scope (non-goals):** a global passive adversary correlating all radio
emissions nationwide; endpoint compromise via a device operating-system exploit; and analysis
that a determined nation-state observer with pervasive sensing could perform.

**Core mitigations:** end-to-end authenticated encryption (content confidentiality/integrity);
Double Ratchet (forward secrecy and post-compromise security); rotating, unlinkable link-layer
identifiers; optional onion routing (relationship anonymity within the mesh); message TTL and
deduplication (flood control); and on-device encryption at rest with optional duress
protections.

---

## 5. System architecture

Project MESH uses a **shared portable core with thin native front-ends**.

```
        ┌──────────────────────────────┐   ┌──────────────────────────────┐
        │        Android app           │   │           iOS app            │
        │   Kotlin + Jetpack Compose   │   │      Swift + SwiftUI         │
        │  (UI, notifications, radio   │   │  (UI, notifications, radio   │
        │   drivers, foreground svc)   │   │   drivers, CoreBluetooth,    │
        │                              │   │   MultipeerConnectivity)     │
        └───────────────┬──────────────┘   └───────────────┬──────────────┘
                        │        UniFFI bindings            │
        ┌───────────────┴───────────────────────────────────┴──────────────┐
        │                     Shared core  (Rust)                            │
        │  identity · Noise handshake · Double Ratchet · sealing/opening ·  │
        │  packet format · store-carry-forward engine · dedup · TTL/expiry · │
        │  channel/group logic · onion routing · persistence (SQLCipher)     │
        └───────────────┬───────────────────────────────────┬──────────────┘
                        │           radio abstraction        │
        ┌───────────────┴──────────┐        ┌────────────────┴─────────────┐
        │  Phone radios (per-OS)    │        │  LoRa bridge (Phase 3)        │
        │  BLE GATT · Wi-Fi Direct/ │        │  BLE/USB link to ESP32/LoRa   │
        │  Aware · Multipeer (iOS)  │        │  node · IN865 865–868 MHz     │
        └───────────────────────────┘        └───────────────────────────────┘
```

### 5.1 The shared Rust core

All security-critical and protocol logic — cryptography, packet construction and parsing, the
routing/relay engine, deduplication, expiry, channel and group semantics, onion routing, and
encrypted persistence — lives in a single **Rust** library. Rationale:

- **Memory safety** for parsing untrusted network input, without a garbage collector.
- **One implementation** of the hard logic, shared byte-for-byte across platforms, eliminating
  a whole class of cross-platform divergence bugs and reducing the audit surface to one
  codebase.
- **No Google/Apple runtime dependency** in the core; it is plain portable Rust.
- **UniFFI** generates safe Kotlin and Swift bindings automatically.

The native layer is deliberately a **"dumb byte pipe"**: it advertises, discovers, connects,
and moves opaque byte buffers to and from the core. It contains no cryptography and no routing
decisions. This mirrors the sound architecture ProtestChat documented, but with the shared
logic in Rust rather than a JavaScript runtime.

### 5.2 Why native UI, not a cross-platform runtime

We deliberately **reject** React Native, Expo, Flutter, and similar runtimes for this project,
despite their development-speed appeal, because:

- Radio behaviour (background BLE scanning/advertising, foreground services, OEM battery-manager
  interactions) is exactly where these runtimes are weakest and most fragile, mediated through
  plugins that lag platform changes.
- Indian device reality is dominated by inexpensive, heavily OEM-customized Android phones
  (Xiaomi, realme, vivo, Oppo, Samsung budget lines) with aggressive background-process
  killers. Reliable background relay on these devices requires precise native control.
- The security-critical logic is already shared (in Rust); the remaining per-platform work is
  UI and radio, which is precisely what should be native.

Full detail: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## 6. Transport layer

This section summarizes; the authoritative treatment, including the iOS background-BLE analysis,
is in [`docs/TRANSPORT.md`](docs/TRANSPORT.md).

### 6.1 Phone-to-phone links

- **Bluetooth Low Energy (BLE) GATT** is the universal baseline: it is the only transport that
  can bridge **Android ↔ iOS** directly. Each device acts as both a GATT peripheral
  (advertising a Project MESH service UUID) and a central (scanning and connecting). Payloads
  are fragmented to fit the negotiated MTU and reassembled by the core.
- **Wi-Fi Direct / Wi-Fi Aware (NAN)** on Android provides much higher throughput for larger
  payloads (e.g. offline map tiles, images) between Android devices.
- **Apple MultipeerConnectivity** provides fast iOS ↔ iOS links using Apple's combined
  Bluetooth/peer-to-peer-Wi-Fi stack, but is Apple-only and cannot bridge to Android.

### 6.2 The iOS constraint (stated plainly)

Apple restricts background Bluetooth severely, and no application can remove these limits:

- A backgrounded iOS app **cannot place a custom service UUID in the main advertising packet**;
  the UUID is moved into a special "overflow" area (a hashed bit in a 128-bit bitmask) that is
  only discoverable by a device **explicitly scanning for that specific UUID**. An Android peer
  that knows the MESH UUID **can** parse this and discover the iOS peripheral — cross-platform
  background discovery is *possible* but higher-latency. *(Corrected per
  `docs/RESEARCH-FINDINGS.md` §3 — an earlier draft wrongly called it impossible.)*
- Overflow advertising is **screen-gated**: it transmits only while the iOS device's screen is
  illuminated. A screen-off backgrounded iOS device goes effectively silent.
- Background scanning is throttled and de-duplicated; since iOS 14 the advertised service set
  cannot be changed while backgrounded.

**Consequence:** iOS devices are strong mesh participants **while the app is in the foreground**,
degraded-but-reachable when backgrounded with the **screen on**, and effectively silent when
backgrounded with the screen off. We do not pretend otherwise. The UX is designed around this:
in gatherings, relief camps, and dead zones, users are guided to keep the app open ("relay
mode"), and iOS-to-iOS clusters use MultipeerConnectivity for much better behaviour. Android
devices, which *can* relay in a foreground service, carry the backbone of the phone-level mesh.

### 6.3 Long-range backbone (Phase 3): LoRa

Phone Bluetooth reaches roughly 10–100 m per hop, which cannot cross a Himalayan valley or a
flooded delta. Genuine rural and disaster coverage requires **LoRa** radio:

- Long range (kilometres line-of-sight; useful non-line-of-sight range in the hundreds of
  metres to low kilometres), very low power, very low bit-rate.
- In India, LoRa operates in the **licence-free 865–868 MHz ISM band** (the **IN865** region in
  LoRaWAN/Meshtastic terminology), subject to the Indian regulator's power and duty-cycle
  limits. This is distinct from the EU (868 MHz) and US (915 MHz) bands and is a
  **hard India-specific design constraint** — see [`docs/HARDWARE-LORA.md`](docs/HARDWARE-LORA.md).
- Implemented as an inexpensive companion node (ESP32 + LoRa module such as those used by
  Meshtastic hardware) that a phone pairs with over BLE/USB. Community-owned solar-powered nodes
  can form a village or valley backbone.

The phone mesh handles dense, urban, and gathering scenarios; the LoRa backbone handles sparse,
long-distance, rural, and disaster scenarios. Messages transit between the two through
phone-plus-node bridges.

---

## 7. Routing protocol

Summary here; wire format and algorithms in
[`docs/ROUTING-PROTOCOL.md`](docs/ROUTING-PROTOCOL.md).

### 7.1 Model

Project MESH uses **store-carry-forward, epidemic-style dissemination with controls**. Every
node keeps a bounded store of unexpired, sealed **envelopes** it has seen. When two nodes meet,
they exchange compact **summary vectors** (Bloom filters / ID digests) of what they hold and
then transfer only the envelopes the other lacks. This "gossip on contact" approach needs no
addresses, routing tables, or topology knowledge — the correct properties for a network with no
stable paths.

### 7.2 Controls against the cost of flooding

Naïve epidemic routing is bandwidth- and storage-hungry and trivially floodable. We bound it:

- **TTL / hop limit:** every envelope carries a maximum hop count and an absolute expiry time;
  both are enforced at every node.
- **Deduplication:** each envelope has a content-derived ID; nodes never re-accept or re-forward
  an ID they have already stored (Bloom filter + bounded seen-set).
- **Store bounds and eviction:** a fixed storage budget with priority-aware eviction (emergency
  SOS traffic is prioritized; low-priority chatter is dropped first).
- **Rate limiting and proof-of-work-lite:** per-peer transfer rate caps; optionally a small
  client puzzle on envelope creation to raise the cost of mass injection (Sybil/flood
  mitigation).
- **Directionality hints (optional):** where a coarse gradient exists (e.g. toward a known LoRa
  gateway or a relief-camp node), spraying can be biased to reduce waste, à la Spray-and-Wait /
  PRoPHET, without requiring full routing state.

### 7.3 Addressing modes

- **Broadcast** ("everyone nearby"): public local messages (e.g. disaster bulletins).
- **Channel:** a passphrase-derived shared key; anyone with the passphrase can read and post; no
  owner or server.
- **Group:** a bounded set of members; the sender seals an individual copy per member.
- **Direct:** one-to-one, end-to-end encrypted between two identities.

---

## 8. Cryptographic design

Summary here; full specification, including handshake transcripts and key schedules, in
[`docs/CRYPTOGRAPHY.md`](docs/CRYPTOGRAPHY.md).

### 8.1 Identity

A user is a **self-generated key pair** — a long-term **Ed25519** signing identity and an
**X25519** key-agreement key — created entirely on-device at first launch. There is **no phone
number, no email, no account, and no registration.** A user's public identity fingerprint can be
shared out-of-band (QR code shown screen-to-screen, or a short spoken safety string) for
in-person verification. Trust is rooted in human verification, never in a server.

### 8.2 Session establishment and message encryption

- **Handshake:** the **Noise Protocol Framework** (the `XX` pattern for mutual authentication
  with identity hiding), using **X25519** for key agreement, **ChaCha20-Poly1305** for AEAD, and
  **BLAKE2s/SHA-256** for hashing. Noise gives a compact, well-analysed, misuse-resistant
  handshake suitable for constrained links.
- **Ongoing 1:1 messaging:** the **Signal Double Ratchet**, providing **forward secrecy** (past
  messages stay safe if a current key leaks) and **post-compromise security** (the session
  self-heals after a key compromise). This is the property ProtestChat's alpha explicitly had
  not yet built; Project MESH treats it as foundational, not optional.
- **Groups:** the Double Ratchet is fundamentally two-party. Sealing one copy per member is O(N)
  and only acceptable for tiny groups; larger groups use **MLS (Messaging Layer Security, RFC
  9420)**, whose TreeKEM gives forward secrecy, post-compromise security, and **logarithmic**
  (not O(N²)) group key-update cost. *(Updated per `docs/RESEARCH-FINDINGS.md`.)*
- **Post-quantum:** the handshake adds **PQXDH** (a post-quantum KEM, ML-KEM/Kyber-1024 class,
  run alongside X25519) to defend against passive "harvest-now, decrypt-later" quantum attacks —
  important because store-and-forward envelopes are long-lived. It does **not** stop an active
  quantum attacker (stated honestly). *(Added per `docs/RESEARCH-FINDINGS.md`.)*
- **Channels:** symmetric keys derived from a passphrase using a memory-hard KDF (**Argon2id**),
  with per-message nonces and AEAD sealing.

### 8.3 Metadata protection

- **Rotating link identifiers:** BLE MAC/identifier randomization and periodic rotation of any
  advertised ephemeral identifier (on the order of every ~15 minutes), from a CSPRNG, so that a
  device cannot be trivially tracked across time and place by its link-layer address.
- **Fixed-size sealed envelopes:** padding envelopes to size buckets so that length does not
  leak message type.
- **Optional onion routing:** for direct messages, a **Sphinx-style** layered-encryption packet
  lets the sender route through several relays such that no single relay learns both source and
  destination — providing **relationship anonymity within the mesh** (subject to the non-goals
  in §3.2 and §4).

### 8.4 At rest

All local storage (messages, keys, contacts) is encrypted at rest (e.g. **SQLCipher**), with the
database key protected by the device keystore / Secure Enclave where available, and an optional
passphrase and **duress/panic** mechanism for the coercive-seizure scenario.

---

## 9. Application features

Project MESH is a **civic platform**, not only a chat app. All features run on the one mesh
substrate. Full detail in [`docs/FEATURES.md`](docs/FEATURES.md).

- **Emergency SOS.** One-tap, high-priority broadcast of a help request with optional coarse
  location, propagated preferentially through the mesh and to any LoRa gateway. Designed for
  disaster and medical emergencies.
- **Disaster bulletin board.** A local, store-and-forward notice board for civic information —
  relief-camp locations, water and medicine availability, road/bridge status, missing-person
  notices. Signed by the poster's key; optionally endorsed by known responder keys.
- **Offline maps.** OpenStreetMap vector tiles rendered with **MapLibre**, fully offline from
  pre-downloaded regional packs (MBTiles/PMTiles). Users can drop and share pins — safe zones,
  relief, hazards, water — over the mesh. No Google Maps, no network calls.
- **Community resource board.** A local "have / need" exchange — food, shelter, transport,
  tools, blood donors — for both everyday rural coordination and disaster relief.
- **Messaging.** Direct, group (bounded membership), passphrase channels, and public
  "everyone nearby" broadcast, all end-to-end encrypted, with store-and-forward delivery.
- **Voice notes and small images** (bandwidth-permitting, transport-dependent), important for
  low-literacy users.

Emergency and safety traffic is prioritized by the routing engine over ordinary messaging.

---

## 10. Localization, accessibility, and user experience

Full detail in [`docs/LOCALIZATION-UX.md`](docs/LOCALIZATION-UX.md). Principles:

- **Many Indian languages, done properly.** First-class support for major Indic scripts and
  languages (Hindi, Bengali, Assamese, Bodo, and others prioritized by the regions served),
  using open fonts (e.g. Noto) with correct complex-script shaping — not merely English with a
  translated string table.
- **Low-literacy first.** Icon-led navigation, large touch targets, voice notes, and optional
  audio prompts, so that reading fluency is not a prerequisite for calling for help or reading a
  bulletin.
- **Low-end devices.** Target inexpensive Android phones (≈2 GB RAM), a small application size,
  and a small persistent footprint; minimize battery drain from scanning and relaying, with
  user-visible controls and clear "relay mode" indication.
- **Offline-first everywhere.** No screen ever blocks on a network call. Every interaction is
  designed for a device that may never touch the internet.

---

## 11. Distribution and the de-Googled requirement

Full detail in [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md).

- **No Google Play Services, no Firebase, no proprietary cloud.** Notifications, background
  execution, and mapping are all implemented without Google's proprietary layer. This is both a
  values requirement and a robustness requirement — the app must run on de-Googled and
  custom-ROM devices and in environments with no Google connectivity.
- **Android delivery** via **F-Droid**, **IzzyOnDroid**, and direct signed **APK**, with an
  optional Play listing built from the same source. **Reproducible builds** so that anyone can
  verify the published binary matches the public source — essential for a security tool.
- **iOS delivery** via the App Store / TestFlight (an unavoidable centralization point that we
  document honestly).

---

## 12. Governance, licensing, and sustainability

Full detail in [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md).

- **Licence.** A copyleft licence (**GPLv3** or **AGPLv3**) is preferred over a permissive one so
  that improvements to public-good infrastructure remain open. (ProtestChat's MIT choice permits
  closed forks; for civic commons we lean copyleft.) Documentation is CC BY-SA 4.0.
- **Stewardship.** Konko Maji stewards the project as a transparent, named, non-profit-style
  initiative with a public mission centred on disaster and rural connectivity — which is also the
  correct legal posture (§13).
- **Open contribution.** Public specification, open issue tracker, and a contributor guide,
  including a **language and framing discipline** (§13) that every contributor agrees to.
- **Sustainability.** Grant funding aligned with disaster-resilience and digital-inclusion
  missions; community-run hardware nodes; no data monetization (there is no data to monetize).

---

## 13. Legal positioning

Full detail in [`docs/LEGAL.md`](docs/LEGAL.md). This is not legal advice; qualified Indian
counsel (e.g. the Software Freedom Law Centre, India) should review before any launch. The
doctrine:

- **Primary identity is disaster and rural civic technology** — genuinely, not as a disguise.
  The features in §9 are real and are the point. Public materials (repository, store listings,
  website) lead with disaster resilience and rural connectivity; the shutdown-resilience
  property is never marketed and never named in project communications.
- **Structural legal shields.** Because the system has **no server, no user database, and no
  personal identifiers**, there is nothing to hand over and no "significant social media
  intermediary" traceability obligation of the kind imposed by the IT Rules, 2021 is triggered
  in the ordinary case. End-to-end encryption is lawful in India. The absence of a central
  operator is a deliberate compliance property, not only a technical one.
- **Alignment, not opposition.** Disaster-resilient, rural-connectivity communication aligns
  with stated national goals (disaster management and rural digital inclusion) and with the
  Supreme Court's recognition in *Anuradha Bhasin* that access to information is
  constitutionally significant.
- **Naming and branding.** Avoid names implying government patronage or colliding with
  government programmes (e.g. "BharatNet"), which raise both confusion and, under the Emblems and
  Names (Prevention of Improper Use) Act, 1950, registration friction.
- **Framing discipline.** Contributors avoid framing the tool as "anti-government" or "for
  circumventing shutdowns"; the consistent language is "communication that works when networks
  are down."

---

## 14. Evaluation and comparison with prior art

| Property | FireChat | Bridgefy | Briar | ProtestChat | Meshtastic | **Project MESH** |
|---|---|---|---|---|---|---|
| Open source | No | Partly | Yes | Yes | Yes | **Yes** |
| Security-first / audited design | No | No (broken) | Yes | Alpha, no audit | Yes | **Yes (goal)** |
| Forward secrecy (ratchet) | No | No | Yes | Not yet | Channel keys | **Yes** |
| Android ↔ iOS bridge | Partial | Yes | Android-first | In progress | Companion | **Yes (BLE)** |
| Honest iOS-background stance | — | — | — | Partial | N/A | **Yes** |
| Long-range (LoRa) backbone | No | No | No | No | **Yes** | **Yes (Phase 3)** |
| India LoRa band (IN865) | — | — | — | — | Yes | **Yes** |
| Civic features beyond chat | No | No | Limited | No | Limited | **Yes** |
| Offline de-Googled maps | No | No | No | No | Basic | **Yes** |
| Indic-language, low-literacy UX | No | No | Partial | No | No | **Yes** |
| Fully de-Googled distribution | No | No | Yes | Partial | Yes | **Yes** |

The comparison is against *stated design goals*; Project MESH is at the design stage, and the
"Yes" entries in its column are commitments to be proven by implementation and audit, not
completed facts.

Planned evaluation once code exists: delivery ratio and latency versus node density in
simulation and field trials; battery cost of relay on representative low-end Android hardware;
LoRa range and throughput in the IN865 band across representative terrain; and an independent
security audit before any general-availability release.

---

## 15. Limitations and open problems

- **iOS background relay is fundamentally limited** by Apple policy and cannot be fully solved
  (§6.2). We mitigate with foreground "relay mode" and iOS-to-iOS MultipeerConnectivity, but
  Android devices necessarily carry the background mesh.
- **Rural distance requires hardware.** Phone-only mesh cannot cover the geographies in scope;
  the LoRa backbone (Phase 3) implies device cost and community deployment effort.
- **DTN delivery is probabilistic**, not guaranteed, and degrades in sparse networks with few
  relays.
- **Battery cost** of continuous scanning and relaying is real and must be tuned per device
  class.
- **Anonymity is bounded** (§3.2, §4): we do not defend against a global passive adversary or
  endpoint compromise.
- **Metadata is not fully hidden**; onion routing reduces but does not eliminate what relays and
  observers can infer.
- **Trust bootstrapping** relies on out-of-band human verification, which has real usability
  cost.
- **Abuse and moderation** in a server-less network are hard; we rely on local blocking,
  key-based reputation, and the absence of global amplification, which is a partial answer.

These are documented, not hidden, in keeping with goal G8.

---

## 16. Roadmap

The phased plan is maintained in [`docs/ROADMAP.md`](docs/ROADMAP.md). In brief:

- **Phase 1 — Core (Android-first).** Rust core; BLE mesh; store-carry-forward engine;
  identity, Noise, Double Ratchet; SOS, disaster bulletin, offline maps, resource board, and
  messaging; Indic localization foundation; F-Droid distribution.
- **Phase 2 — Reach and hardening.** iOS front-end (with the honest background model); more
  languages; performance tuning for low-end devices; onion-routing option; independent security
  audit.
- **Phase 3 — Hybrid (LoRa backbone).** LoRa companion-node bridge in the IN865 band; reference
  solar node design; open contribution model for community node builders.
- **Phase 4 — Ecosystem.** Community node networks, governance maturity, reproducible-build
  verification, and field partnerships with disaster-response and rural-connectivity
  organizations.

Shutdown-resilience is present from Phase 1 as an emergent property, never as a stated feature.

---

## 17. Conclusion

India's communication failures in coverage gaps, disasters, and shutdowns are one problem
wearing three faces: dependence on centralized infrastructure. Project MESH answers that
problem with a decentralized, server-less, end-to-end encrypted mesh that works phone-to-phone
today and across a long-range LoRa backbone tomorrow, wrapped in a genuine civic-resilience
feature set, built for India's languages and devices, distributed free of proprietary
dependencies, and positioned honestly and lawfully as the disaster and rural-connectivity
infrastructure it truly is. The design is deliberately candid about what is hard and what is
impossible; the value of a resilience tool lies entirely in whether its claims are true. The
next step is implementation of Phase 1 and independent scrutiny of both this design and the code
that follows.

---

## 18. References

A consolidated, linked bibliography — DTN routing, Noise, Double Ratchet, Sphinx/Loopix/Nym, the
Bridgefy analysis, Briar, Meshtastic, MapLibre/OpenStreetMap, the Indian LoRa band, the
Temporary Suspension Rules 2017, *Anuradha Bhasin v. Union of India* (2020), shutdown trackers,
and the IT Rules 2021 — is maintained in [`docs/REFERENCES.md`](docs/REFERENCES.md).
