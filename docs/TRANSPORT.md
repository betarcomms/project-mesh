# Transport Layer

Authoritative treatment of Project MESH's physical and link transports. Companion to
`WHITEPAPER.md` §6.

---

## 1. Transport matrix

| Transport | Reaches | Typical range | Throughput | Notes |
|---|---|---|---|---|
| BLE GATT | Android ↔ Android, Android ↔ iOS, iOS ↔ iOS | ~10–100 m | Low (kbit/s–100s kbit/s) | Universal baseline; only cross-OS bridge |
| Wi-Fi Direct / Aware (NAN) | Android ↔ Android | ~50–200 m | High (Mbit/s) | For bulk payloads (map packs, images) |
| MultipeerConnectivity | iOS ↔ iOS | ~10–70 m | Medium–High | Apple-only; best iOS behaviour |
| LoRa (Phase 3) | node ↔ node | ~0.5–10+ km | Very low (0.3–27 kbit/s) | Long-range backbone; companion hardware |

The core treats all of these as interchangeable links carrying opaque frames.

## 2. BLE: the universal baseline

Every device is simultaneously:

- a **GATT peripheral**, advertising a Project MESH **service UUID** and exposing a
  characteristic for frame exchange; and
- a **GATT central**, scanning for that UUID, connecting to discovered peers, and exchanging
  frames.

### 2.1 Fragmentation

BLE payloads are constrained by the negotiated **ATT MTU** (commonly ~185–517 bytes after
negotiation). The core fragments envelopes into MTU-sized chunks with sequence/length headers
and reassembles them, tolerating loss and out-of-order arrival.

### 2.2 Connection management

- Short-lived connections: connect, exchange summary vectors, transfer missing envelopes,
  disconnect — to conserve battery and free slots (peripherals support a limited number of
  simultaneous centrals).
- Backoff and fairness so a dense crowd does not thrash the radio.

## 3. The iOS background problem (in full)

This is the single most important transport constraint and is stated without euphemism.

> **Verified against Apple's Core Bluetooth Background Processing Guide and independent
> reverse-engineering (David G. Young, "Hacking the Overflow Area"). One earlier claim here was
> too strong and has been corrected — see `RESEARCH-FINDINGS.md` §3.**

### 3.1 What Apple does

- When an iOS app is **backgrounded**, Core Bluetooth **removes the app's service UUID from the
  main advertisement** and relocates it into a special **"overflow" area** — an Apple
  manufacturer-specific packet (manufacturer code `0x004C`, type `0x01`) carrying a **128-bit
  hashed bitmask** in which each advertised service UUID sets one bit position.
- Per Apple's documentation, UUIDs in the overflow area **can only be discovered by a device
  explicitly scanning for that specific UUID** — a *generic* scan cannot find them, because the
  UUID exists only as a hashed bit, not a readable value.
- **Correction (important):** an **Android** central **can** discover a backgrounded iOS
  peripheral, by parsing that Apple overflow manufacturer packet and testing the relevant bit in
  the bitmask. Cross-platform background discovery **is possible** — provided the scanner already
  knows the specific service UUID to look for. (An earlier version of this doc overstated it as
  impossible.)
- **Screen-gated:** overflow-area advertisements are only transmitted while the sending device's
  **screen is illuminated** (screen on; the device need not be unlocked or the app foregrounded).
  A screen-off backgrounded iOS device effectively stops advertising to the mesh.
- Background **scanning** is throttled: scans are slowed, `AllowDuplicates` is ignored, and
  repeated discoveries are coalesced — increasing discovery latency. The background local name is
  dropped. Since **iOS 14**, an app **cannot change which services it advertises while
  backgrounded** (the set must be configured in the foreground).
- Background execution time is limited and at the OS's discretion.

### 3.2 Consequences (precise)

- An iOS device with the app **in the foreground** (screen on, app open) is a **full mesh
  participant** and bridges to Android reliably.
- An iOS device **backgrounded but screen-on** still advertises via the overflow area and **can**
  be discovered by an Android peer that scans for the known MESH UUID — but with **higher latency
  and lower reliability**, and it cannot initiate much on its own.
- An iOS device **backgrounded and screen-off** is effectively **silent** to the mesh.
- No entitlement, API, or trick removes the throttling, the screen-gating, or the
  overflow-encoding. It is platform policy — the limits are real, just more nuanced than
  "impossible."

### 3.3 How Project MESH designs around it

- **Relay mode UX.** In gatherings, relief camps, or dead zones, the app guides iOS users to
  keep it open and screen-on ("relay mode"), clearly indicating that the phone is actively
  helping the network. This is a deliberate, honest UX pattern, not a workaround claim.
- **iOS clusters use MultipeerConnectivity.** Among iOS devices, MPC provides much better
  peer-to-peer behaviour than raw background BLE.
- **Android carries the background backbone.** Android devices, via a foreground service, relay
  in the background and act as the persistent bridge between iOS foreground participants.
- **No false promises.** Store listings and docs state plainly that continuous background relay
  is an Android strength and an iOS limitation.

## 4. Wi-Fi Direct / Aware (Android)

For payloads too large for comfortable BLE transfer — offline **map tile packs**, images, voice
notes — Android devices negotiate a higher-throughput **Wi-Fi Direct** or **Wi-Fi Aware (NAN)**
link, transfer the bulk data, and tear it down. Discovery can be bootstrapped over BLE.

## 5. LoRa backbone (Phase 3)

Phone Bluetooth cannot cross rural distances; LoRa can. Full hardware and regulatory detail is
in `HARDWARE-LORA.md`. Transport-relevant points:

- A phone pairs with a **companion LoRa node** (ESP32 + LoRa radio) over BLE or USB-serial.
- The node speaks LoRa to other nodes, forming a long-range backbone (village, valley, camp).
- The core bridges envelopes between the phone mesh and the LoRa mesh at any phone-plus-node
  point, respecting LoRa's tiny bit-rate by forwarding only small, high-priority payloads (SOS,
  bulletins, text) over the radio and keeping bulk data on Wi-Fi/BLE.
- **India band:** LoRa here uses the licence-free **865–868 MHz (IN865)** band with the Indian
  regulator's power and duty-cycle limits — a hard constraint distinct from EU/US bands.

## 6. Power and OEM behaviour

- Continuous scan/advertise is the dominant battery cost; duty-cycling, adaptive scan windows,
  and connection batching are tuned per device class.
- Indian budget Android devices ship aggressive background-process killers (Xiaomi/MIUI, realme,
  vivo, Oppo/ColorOS, Samsung). The app detects these and guides users through the
  vendor-specific battery-whitelisting steps needed for reliable background relay.
