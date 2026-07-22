# Application Features

Companion to `WHITEPAPER.md` §9. Project MESH is a civic-resilience **platform**, not only a
messenger. Every feature runs on the single mesh substrate (`ROUTING-PROTOCOL.md`) and uses the
same cryptography (`CRYPTOGRAPHY.md`).

---

## 1. Emergency SOS

**Purpose:** get a help request out when nothing else works.

- **One-tap** SOS composes a high-priority broadcast envelope (`class = SOS`).
- Optional **coarse location** (a pin the user confirms; precision is user-controlled for
  safety).
- Optional **category** (medical / trapped / fire / violence / other) via icons, so low-literacy
  users can specify without typing.
- Routing engine gives SOS **top priority** in contact exchange and in store eviction, and
  forwards it preferentially to any **LoRa gateway** for long-range escape from a dead zone.
- Nearby users see an SOS with distance/direction (if location shared) and can acknowledge; an
  ACK travels back through the mesh.

**Honest limit:** delivery is best-effort; SOS reaches whoever the mesh can carry it to, which
depends on node density. It is a resilience layer, not a replacement for emergency services
where they exist.

## 2. Disaster bulletin board

**Purpose:** spread trustworthy local civic information.

- A **store-and-forward notice board** of local bulletins: relief-camp locations, food/water/
  medicine availability, road and bridge status, shelter openings, missing-person notices.
- Each bulletin is **signed by the poster's key**; bulletins from **known responder keys**
  (e.g. a verified NGO or camp coordinator) can be visually distinguished and endorsed.
- Bulletins carry TTL so stale information ages out.
- Readable by anyone nearby (public class), so information spreads without requiring prior
  contact.

## 3. Offline maps

**Purpose:** situational awareness with no internet and no Google.

- **OpenStreetMap** vector tiles rendered by **MapLibre GL Native**, fully offline from
  pre-downloaded **regional tile packs** (MBTiles / PMTiles). No network calls, no Google Maps.
- Tile packs for at-risk regions can be bundled or side-loaded, and shared **device-to-device
  over Wi-Fi Direct** (bulk transport) so one downloaded map seeds a whole area.
- Users drop and share **pins over the mesh**: safe zones, relief points, water, hazards,
  blocked roads, medical help. Pins are envelopes like any other and propagate through the mesh.

## 4. Community resource board

**Purpose:** match needs and offers locally.

- A local **"have / need"** exchange: food, shelter, transport, tools, blood donors, charging
  points, labour.
- Useful in **ordinary rural life** (not only disasters) — which is deliberate: the platform
  earns daily use so it is present and understood when a crisis hits.
- Entries are signed, categorized by icon, and TTL-bounded.

## 5. Messaging

- **Direct** (1:1), end-to-end via Double Ratchet, optional onion routing.
- **Group** (bounded membership), per-member sealed copies.
- **Channel** (passphrase-derived key), owner-less, for open local topics.
- **Broadcast** ("everyone nearby"), public local messages.
- All messages are store-and-forward; recipients receive them whenever the mesh delivers,
  possibly after a delay.

## 6. Voice notes and small images

- **Voice notes** are first-class, because they serve **low-literacy** users directly.
- **Small images** (compressed) where transport allows; large media prefers Wi-Fi Direct.
- Both respect the routing engine's priority and size rules; over LoRa, media is not forwarded
  (bit-rate too low) — only text/SOS/bulletins.

## 7. Cross-cutting behaviour

- **Priority:** SOS > bulletin > resource/map > direct/group > channel/broadcast chatter, in
  both transfer order and eviction.
- **Everything is an envelope:** SOS, pins, bulletins, and messages share one wire format and
  one routing engine, keeping the system small and auditable.
- **Everything works offline-first:** no feature blocks on connectivity.
