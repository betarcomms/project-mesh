# References and Prior Art

Companion to `WHITEPAPER.md` §18. Grouped by topic. Where a formal citation is known it is
given; otherwise a descriptive pointer is provided. Verify current URLs and editions.

---

## Delay-tolerant and mesh routing

- K. Fall, "A Delay-Tolerant Network Architecture for Challenged Internets," SIGCOMM, 2003.
- A. Vahdat and D. Becker, "Epidemic Routing for Partially Connected Ad Hoc Networks," Duke
  Univ. Tech. Report CS-2000-06, 2000.
- T. Spyropoulos, K. Psounis, C. Raghavendra, "Spray and Wait: An Efficient Routing Scheme for
  Intermittently Connected Mobile Networks," WDTN, 2005.
- A. Lindgren, A. Doria, O. Schelén, "Probabilistic Routing in Intermittently Connected
  Networks" (PRoPHET), SIGMOBILE MC2R, 2003.

## Cryptographic constructions

- T. Perrin, "The Noise Protocol Framework," https://noiseprotocol.org
- M. Marlinspike and T. Perrin, "The Double Ratchet Algorithm," Signal, 2016.
- M. Marlinspike and T. Perrin, "The X3DH Key Agreement Protocol," Signal, 2016.
- G. Danezis and I. Goldberg, "Sphinx: A Compact and Provably Secure Mix Format," IEEE S&P, 2009.
- A. Piotrowska et al., "The Loopix Anonymity System," USENIX Security, 2017.
- Nym mixnet documentation, https://nymtech.net
- Argon2, A. Biryukov, D. Dinu, D. Khovratovich, password-hashing competition winner, 2015.
- D. J. Bernstein, "ChaCha20 and Poly1305"; "Curve25519 / Ed25519."

## Consumer mesh apps and analyses

- M. R. Albrecht, J. Blasco, R. B. Jensen, L. Mareková, "Mesh Messaging in Large-Scale
  Protests: Breaking Bridgefy," CT-RSA, 2021.
- Briar, https://briarproject.org (open-source, security-focused mesh messenger).
- Bitchat, open-source Bluetooth-mesh messenger (https://github.com/permissionlesstech/bitchat).
- Berty, https://berty.tech (decentralized messenger, BLE + internet).
- ProtestChat (ni5arga), https://github.com/ni5arga/protestchat (React Native/Expo BLE-mesh
  alpha; see its `FORWARD-SECRECY.md` and `docs/THREAT-MODEL.md`).
- FireChat (Open Garden), historical mesh messenger; mesh support discontinued c. 2018–2019.

## Long-range radio mesh

- Meshtastic, https://meshtastic.org (LoRa mesh; supports the IN865 region). Firmware:
  https://github.com/meshtastic
- Semtech LoRa / SX126x documentation (PHY reference).
- LoRa Alliance regional parameters (band plans, including IN865).

## Mapping (offline, de-Googled)

- OpenStreetMap, https://www.openstreetmap.org
- MapLibre GL Native, https://maplibre.org
- MBTiles specification; PMTiles, https://protomaps.com/docs/pmtiles

## Platform and tooling

- UniFFI (Mozilla), https://mozilla.github.io/uniffi-rs/
- SQLCipher, https://www.zetetic.net/sqlcipher/
- Apple Core Bluetooth background behaviour, Apple Developer documentation, "Core Bluetooth
  Background Processing" (service-UUID overflow area; background scan throttling).
- Android BLE, Wi-Fi Direct, Wi-Fi Aware (NAN), Android Developer documentation.
- F-Droid reproducible builds, https://f-droid.org/docs/Reproducible_Builds/

## India: shutdowns, law, and connectivity

- *Anuradha Bhasin v. Union of India*, (2020) 3 SCC 637 (Supreme Court of India), internet
  access and proportionality of shutdowns.
- Telecommunications (Temporary Suspension of Services) Rules, 2024 (in force 22 Nov 2024, under
  the Telecommunications Act, 2023), supersedes the Temporary Suspension of Telecom Services
  (Public Emergency or Public Safety) Rules, 2017 (Indian Telegraph Act, 1885), which is
  historical only.
- Telecommunications Act, 2023 (partial commencement 26 June 2024; §20 suspension/intercept, §29
  identity duty).
- Information Technology Act, 2000 (incl. §69A); Information Technology (Intermediary Guidelines
  and Digital Media Ethics Code) Rules, 2021.
- Unlawful Activities (Prevention) Act, 1967 (UAPA).
- Emblems and Names (Prevention of Improper Use) Act, 1950.
- Software Freedom Law Centre, India, internet shutdown tracker,
  https://internetshutdowns.in ; https://sflc.in
- Access Now / #KeepItOn, annual "internet shutdowns" reports (India consistently ranked
  highest globally).
- WPC (Wireless Planning & Coordination Wing) / TEC (Telecommunication Engineering Centre),
  Department of Telecommunications, India ISM-band and 865–868 MHz usage rules (GSR 853(E), 10 Dec 2021).

## Indian rural / community networks (context and prior art)

- Gram Marg rural broadband research, IIT Bombay.
- Community mesh network deployments in Ladakh and other remote regions (see Digital Resilience
  Hub and related documentation).

## Verified primary sources (from the deep-research pass, see `RESEARCH-FINDINGS.md`)

These were fetched and adversarially verified against primary text:

- **RFC 9420**, "The Messaging Layer Security (MLS) Protocol," IETF, July 2023, group key
  establishment with FS + PCS, TreeKEM logarithmic scaling.
- **PQXDH**, "The PQXDH Key Agreement Protocol," Signal, Rev 3 (2023-05-24, upd. 2024-01-23),
  https://signal.org/docs/specifications/pqxdh/, post-quantum X3DH extension (ML-KEM/Kyber-1024).
- **The Double Ratchet Algorithm**, Perrin & Marlinspike, Revision 4, 2025-11-04,
  https://signal.org/docs/specifications/doubleratchet/, 1:1; integrates PQXDH.
- **ML-KEM**, NIST FIPS 203 (post-quantum KEM standard); adopted by Signal, iMessage, Session.
- **Gazette of India, G.S.R. 853(E)**, 10 December 2021, "Use of Low Power Equipment in the
  Frequency Band 865–868 MHz for Short Range Devices (Exemption from Licence) Rules, 2021,"
  Ministry of Communications, WPC Wing (thc.nic.in). 25 mW e.r.p., 1% duty cycle, FHSS.
- **Telecommunications (Temporary Suspension of Services) Rules, 2024**, notified 22 Nov 2024
  under the Telecommunications Act, 2023; supersedes the 2017 Rules (SFLC.in; Internet Freedom
  Foundation; SCC Online).
- **Telecommunications Act, 2023**, partial commencement 26 June 2024 (§20 suspension/intercept;
  §29 identity duty). Gazette + DoT.
- **Apple, Core Bluetooth Background Processing Guide**, overflow-area behaviour; corroborated by
  David G. Young, "Hacking the Overflow Area," davidgyoungtech.com / github.com/davidgyoung/
  ios-overflow-area.
- BLE physical-layer fingerprinting: H. Givehchian et al., "Evaluating Physical-Layer BLE Location
  Tracking Attacks on Mobile Devices," **IEEE S&P (Oakland) 2022**, CFO/I-Q fingerprinting defeats
  MAC randomization.
- M. R. Albrecht et al., "Breaking Bridgefy, again: Adopting libsignal is not enough," **USENIX
  Security 2022**, integration bugs (TOCTOU, compress-then-encrypt) defeat Signal-protocol use.
- **Reticulum**, reticulum.network manual v1.4.0; github.com/markqvist/Reticulum; FOSDEM 2026
  sessions (spec/governance transition); Rust reimplementation `reticulum-rs`.
- **Session V2 protocol** (late 2025), ML-KEM + reinstated PFS + onion routing.
- **UniFFI / Mozilla application-services**, cross-platform Rust bindings (JNA rationale); **Crux**
  (redbadger/crux), shared Rust core framework.

---

*Note:* entries above the "Verified primary sources" heading are a mix of pinned citations and
pointers; before publication each pointer should be re-checked for current URL, edition, and
accuracy. Forward-looking items (Session V2, Reticulum governance) are current as of
late-2025/early-2026 and should be re-verified.
