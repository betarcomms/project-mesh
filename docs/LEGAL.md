# Legal Positioning

Companion to `WHITEPAPER.md` §13.

> **This is not legal advice.** It is the project's positioning doctrine and a good-faith
> summary of relevant considerations. Qualified Indian legal counsel — for example the
> **Software Freedom Law Centre, India (SFLC.in)** — must review the project, its materials, and
> its governance before any public launch. Laws and rules change; verify current text.

---

## 1. The doctrine in one line

**Project Mesh is genuine disaster-resilience and rural-connectivity civic infrastructure. That
is its true primary purpose, its public identity, and its legal footing. Its resilience during
network shutdowns is an emergent property of good engineering — never a marketed feature.**

## 2. Why framing is load-bearing

The *same technology* can be described as "a tool to defeat government internet blocks" or as "a
tool that keeps communities connected during floods, in remote valleys, and whenever networks
are down." The first framing manufactures legal and political risk; the second is accurate,
sincere, and aligns with recognized public interests. The physics is identical; the words are
not. We choose — truthfully — the civic framing.

| Never say | Always say |
|---|---|
| "Beat government internet blocks" | "Works when networks are down" |
| "Anti-shutdown / protest tool" | "Disaster-resilient communication" |
| "Circumvent censorship" | "Connectivity for disconnected and rural areas" |
| "For activists" | "For communities, first responders, and remote regions" |

## 3. Structural legal shields (properties of the design)

These are engineering choices that *also* reduce legal exposure:

- **No server, no operator.** There is no central service, so there is no party who can be
  compelled to produce data, and no data held to produce.
- **No user database, no personal identifiers.** No phone numbers, no emails, no accounts — there
  is nothing to collect, retain, or disclose.
- **Intermediary status / traceability.** The **IT (Intermediary Guidelines and Digital Media
  Ethics Code) Rules, 2021** impose duties on intermediaries; **Rule 4(2)** requires *significant
  social media intermediaries* to enable identification of the **"first originator"** of a
  message. That rule is being litigated (WhatsApp/Facebook in the Delhi High Court; a FOSS
  developer, Praveen Arimbrathodiyil, in the Kerala High Court) as breaking E2EE and the Article
  21 privacy right. A **server-less, operator-less, account-less** local mesh has **no first
  originator to produce and no service backend** to which these duties attach, and does not
  operate as a significant social media intermediary in the ordinary case. (Counsel to confirm as
  applied — this is the reasoned position, not an adjudicated one; see `RESEARCH-FINDINGS.md` §5.)
- **Encryption is lawful.** India does not ban end-to-end encryption. Framing it as protecting
  citizens' private data is both true and standard security practice.
- **New risk vector — Telecom Act 2023 §29 (anti-anonymity).** The Telecommunications Act, 2023
  (partly in force from 26 June 2024) imposes at **§29** a duty on users **not to furnish false
  identity information** to avail telecommunication services. **If** this is later applied to
  internet/OTT services (whether OTT/E2EE apps fall under the Act is currently **legally
  ambiguous** — a Minister's Dec 2023 verbal exclusion is not binding), it could pressure
  anonymous communication. Mesh's design (no identity to furnish at all) sits awkwardly against a
  future anti-anonymity reading; this is flagged for counsel, not resolved.

## 4. Alignment with recognized interests

- **Disaster management** is a stated national priority; a tool that keeps communication alive
  when infrastructure is destroyed serves it directly.
- **Rural digital inclusion** is a stated national goal; connecting under-served regions aligns
  with it.
- **Constitutional recognition.** In *Anuradha Bhasin v. Union of India* (2020), the Supreme
  Court held that access to information through the internet is protected under Article 19 and
  that restrictions must satisfy proportionality and cannot be indefinite. Resilient
  communication is thus a legitimate civic interest, not a subversive one.

## 5. Relevant legal instruments (for counsel review)

> **Updated to the current framework — verified; see `RESEARCH-FINDINGS.md` §5.** The 2017
> shutdown rules have been **superseded**.

- **Telecommunications Act, 2023** — the new governing statute (partly in force from **26 June
  2024**), replacing the Indian Telegraph Act, 1885 framework. **§20** empowers the Union to take
  possession of / suspend / intercept telecom services and messages on public-emergency/safety
  grounds. **§29** imposes a duty on users **not to furnish false identity information** to avail
  services (the anti-anonymity risk flagged in §3). Whether **OTT/E2EE services** fall under the
  Act is **legally ambiguous** and unresolved.
- **Telecommunications (Temporary Suspension of Services) Rules, 2024** — notified and **in force
  from 22 November 2024** under the Telecom Act 2023, **superseding** the Temporary Suspension of
  Telecom Services (Public Emergency or Public Safety) Rules, 2017. A single order is capped at
  **15 days** (Rule 3(2)(b)(iii)) and must specify the **geographical region**; the **Review
  Committee (Rule 6) is executive-only** (Home + Law officials), with no judicial/public
  oversight. In practice, "new language, same shutdowns."
- **Anuradha Bhasin v. Union of India, (2020) 3 SCC 637** — the governing precedent: shutdown
  orders must be **legal, necessary, proportionate, time-bound, reasoned, published, and
  reviewable**; **indefinite shutdowns are unconstitutional.**
- **Section 163, Bharatiya Nagarik Suraksha Sanhita, 2023** (formerly §144 CrPC) — local
  prohibitory orders sometimes used alongside shutdowns.
- **Information Technology Act, 2000** (incl. **§69A** blocking) and the **IT Rules, 2021** (incl.
  **Rule 4(2)** traceability — see §3).
- **Unlawful Activities (Prevention) Act (UAPA)** — the risk to *avoid* by not positioning the
  tool as anti-state; a genuine disaster/rural civic tool is the opposite of an unlawful
  activity, but careless "anti-government" framing invites scrutiny. Hence the framing
  discipline.
- **Emblems and Names (Prevention of Improper Use) Act, 1950** — restricts names implying
  government patronage or misusing "India"/"Bharat"; a reason to avoid names colliding with
  government programmes (e.g. "BharatNet") in addition to the confusion such names cause.
- **Spectrum for LoRa** — Project Mesh uses **licence-free** bands (BLE/Wi-Fi at 2.4 GHz; LoRa at
  **IN865, 865–868 MHz** per GSR 853(E), 2021), but type-approval and spectrum compliance for LoRa
  deployment is the deployer's responsibility (`HARDWARE-LORA.md`).

## 6. Naming and branding rules

- Avoid names implying **government endorsement** or colliding with **government programmes**
  (BharatNet, Digital India, etc.) — both for confusion and for Emblems-and-Names-Act friction.
- Avoid names with **political-symbol** connotations (e.g. party symbols), which compromise the
  neutral civic posture.
- Prefer a neutral, mission-clear identity.

## 7. Operational guidance

- Register a **neutral non-profit** with a disaster/rural mission on the record (`GOVERNANCE.md`).
- Keep **all public materials** (repository, store listings, website, talks) consistently in the
  civic framing.
- Brief every contributor on the **framing discipline**; a single careless "smash the shutdown"
  post undermines the posture built everywhere else.
- Obtain **counsel review** before launch and keep it current as the Telecommunications Act,
  2023 rules and other instruments evolve.

## 8. Honesty caveat

Positioning reduces risk; it does not eliminate it, and it is not a substitute for legal advice.
The strongest protection is that the tool is **genuinely and primarily** what it says it is:
real, valuable disaster-resilience and rural-connectivity infrastructure whose civic worth is
self-evident.
