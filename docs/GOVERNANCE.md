# Governance, Licensing, and Sustainability

Companion to `WHITEPAPER.md` §12.

---

## 1. Stewardship

Project Mesh is stewarded by **Konko Maji** as a transparent, research-and-open-source
initiative with a **public mission centred on disaster resilience and rural connectivity**. The
recommended legal form is a **non-profit** structure (e.g. a Section-8 company or a registered
society/trust in India) with named, accountable stewardship. A neutral, mission-clear non-profit
is also the correct **legal posture** (see `LEGAL.md`).

## 2. Licensing

### 2.1 Source code — copyleft preferred
- Recommended: **GPLv3** or **AGPLv3**. Copyleft keeps improvements to public-good infrastructure
  open, preventing closed proprietary forks that take without giving back.
- Contrast: ProtestChat uses **MIT**, which permits closed-source derivatives. For civic commons
  we lean copyleft; the final choice is to be confirmed with counsel and community input,
  weighing whether commercial closed forks should be permitted (MIT) or not (GPL/AGPL).
- **AGPL** specifically closes the "network-service" loophole — relevant if any optional
  server-side helper is ever written (there is none in the core design).

### 2.2 Documentation
- **CC BY-SA 4.0** for all documentation (this repository).

### 2.3 Hardware / firmware
- Reference node designs and firmware profiles under an appropriate open hardware/firmware
  licence, interoperating with and contributing upstream to **Meshtastic** where practical.

## 3. Contribution model

- **Open specification and issue tracker.** Design happens in the open.
- **Contributor guide** covering code standards, the Rust-core boundary, security-review
  expectations, and the **framing discipline** below.
- **Security disclosure policy:** a responsible-disclosure process and, before any
  general-availability release, an **independent external security audit** (a hard gate).
- **Localization workflow:** community translation with a safety-term glossary
  (`LOCALIZATION-UX.md`).

## 4. Framing discipline (required of all contributors)

Because positioning is load-bearing (`LEGAL.md`), every contributor agrees to consistent
language in code, docs, commits, issues, and public communication:

- **Do** describe the project as disaster-resilience and rural-connectivity civic technology,
  and as "communication that works when networks are down."
- **Do not** describe it as "anti-government," "for defeating shutdowns," or "for protest
  circumvention," and do not name shutdown-circumvention as a feature.
- This is not about hiding capability; it is about accurately leading with the genuine primary
  purpose and not manufacturing legal risk with careless words.

## 5. Sustainability

- **Funding:** grants aligned with disaster-resilience, digital-inclusion, and open-source
  public-infrastructure missions; individual and institutional donations. No venture model that
  would pressure the project away from its ethos.
- **No data monetization:** there is no user data to monetize — by design there is no server,
  no account, and no directory.
- **Community infrastructure:** LoRa relay nodes are community-owned and community-funded,
  distributing both cost and control.
- **Institutional partners:** disaster-management bodies, rural-connectivity programmes, and
  digital-rights organizations as deployment and review partners.

## 6. Decision-making

- Technical decisions via open proposals (design docs / RFCs) and maintainer review.
- Security-relevant changes require review against the threat model (`THREAT-MODEL.md`).
- Roadmap (`ROADMAP.md`) is public and revised in the open.
