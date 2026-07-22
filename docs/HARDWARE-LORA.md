# Hardware and LoRa Backbone (Phase 3)

Companion to `WHITEPAPER.md` §6.3. Phone Bluetooth cannot cover rural distances; a long-range
radio backbone can. This document specifies the LoRa approach and the **India-specific**
regulatory constraints.

> **Regulatory disclaimer:** this is an engineering overview, not legal or spectrum-compliance
> certification. Any deployment must comply with the current rules of India's spectrum regulator
> (WPC / TEC / DoT) and use type-approved equipment where required. Verify current limits before
> building or deploying.

---

## 1. Why LoRa

- **Long range:** kilometres line-of-sight; useful non-line-of-sight range from hundreds of
  metres to low kilometres depending on terrain and settings.
- **Very low power:** nodes can run on small solar panels and batteries.
- **Very low bit-rate** (≈0.3–27 kbit/s): fine for text, SOS, and bulletins; unsuitable for
  media (which stays on BLE/Wi-Fi).
- **Mature open ecosystem:** the **Meshtastic** project already implements LoRa mesh on
  inexpensive boards and supports India's frequency region.

## 2. India band: 865–868 MHz (IN865)

> **Verified against the primary source** — Gazette of India **G.S.R. 853(E), 10 December 2021**,
> "Use of Low Power Equipment in the Frequency Band 865–868 MHz for Short Range Devices
> (Exemption from Licence) Rules, 2021" (Ministry of Communications, WPC Wing). See
> `RESEARCH-FINDINGS.md` §4.

- India permits **licence-free** low-power short-range-device use across the **865–868 MHz**
  sub-band (GSR 853(E) delicensed it, **superseding** the older 865–867 MHz RFID rules of 2005).
  In LoRaWAN/Meshtastic terms this is the **IN865** region.
- This is **different from EU868 and US915** — a hard constraint. Hardware and firmware must be
  configured to IN865 within 865–868 MHz; using EU/US band plans in India is non-compliant.
- **Exact limits from the notification:**
  - **Non-Specific Short Range Devices** (the class Mesh user/relay nodes fall in):
    **25 mW e.r.p.**, **1% duty cycle**, **FHSS**, **max occupied bandwidth 50 kHz for 58 or more
    hop channels**. The duty cycle applies to the entire transmission, not per hop channel.
    (Standard EN 300 220 referenced.)
  - **Tracking/Tracing & Data-Acquisition Devices** (higher-power class): up to **500 mW e.r.p.**
    with **Adaptive Power Control required**, duty cycle **10%** for network access points /
    **2.5%** otherwise, **200 kHz** bandwidth.
- **Conditions:** no licence for compliant devices, but only on a **non-interference,
  non-protected, shared, non-exclusive** basis, and equipment **must be type-approved** by the
  Central Government. Legal basis: Indian Telegraph Act 1885 + Indian Wireless Telegraphy Act 1933.
- **Design impact:** the **1% duty cycle** on the 25 mW class is a hard throughput ceiling — it
  reinforces "small, high-priority messages only over LoRa" (`ROUTING-PROTOCOL.md` §6). The
  routing engine enforces conservative duty-cycling so the backbone stays within limits. Any
  500 mW operation is a different device class requiring Adaptive Power Control and type approval.

## 3. Node design (reference)

A companion node, not a phone:

- **MCU + radio:** ESP32-class board with a LoRa transceiver (e.g. SX126x), of the type used by
  Meshtastic hardware (Heltec, RAK, LILYGO T-Beam, etc.).
- **Antenna:** tuned for 865–868 MHz.
- **Power:** battery + small solar for unattended operation; low sleep-current firmware.
- **Phone link:** BLE (or USB-serial) to a paired phone running Project Mesh.
- **Optional GPS** on fixed relay nodes for positioning/time (not required on user nodes).

Roles:

- **User node:** carried/owned by an individual; bridges that person's phone mesh to LoRa.
- **Relay node:** fixed, solar-powered, placed for coverage (ridge line, tower, camp), forming a
  village/valley backbone.
- **Gateway node:** a relay that also has occasional internet (e.g. VSAT/where available) can act
  as an egress point — strictly optional and never required.

## 4. Phone ↔ node bridging

- The phone pairs with its node over BLE/USB; the Project Mesh core treats the node as just
  another `MeshTransport` (see `ARCHITECTURE.md` §3).
- Over the LoRa link the core forwards **only small, high-priority classes** (SOS, bulletins,
  short text). Bulk data never touches LoRa.
- Envelopes cross freely between the phone mesh and the LoRa mesh at any phone-plus-node point,
  so a message can originate on a phone, hop across a valley over LoRa, and be delivered to
  another phone on the far side.

## 5. Firmware strategy

- **Preferred:** interoperate with / extend the **Meshtastic** firmware and its IN865 support,
  rather than reinventing LoRa PHY/MAC, contributing upstream where useful.
- Project Mesh envelopes are carried as opaque payloads within the LoRa transport; the sealing
  and routing semantics remain those defined in `CRYPTOGRAPHY.md` and `ROUTING-PROTOCOL.md`.
- A minimal custom firmware profile may be defined for Mesh-specific priority handling, staying
  within band limits.

## 6. Deployment model

- **Community-owned:** nodes are built and maintained by local communities, NGOs, and
  volunteers — matching the civic, decentralized ethos and avoiding any central operator.
- **Documentation and kits:** open build guides, bills of materials, and recommended
  configurations for the IN865 band, so a community can stand up a backbone without deep RF
  expertise.
- **Cost:** individual nodes are inexpensive (roughly the price range of existing Meshtastic
  boards); solar relay nodes cost more but are shared infrastructure.

## 7. Honest limitations

- LoRa's tiny bit-rate means the backbone is for **critical small messages**, not general media.
- Range is **highly terrain-dependent**; real coverage requires siting relay nodes thoughtfully.
- Hardware implies **cost and effort**, which is why Phase 3 follows a working phone-only Phase
  1–2 rather than gating the whole project on radios.
- Regulatory compliance is the **deployer's responsibility**; the project provides conservative
  defaults and documentation, not a legal guarantee.
