# Design Brief: Betar

This is the brief for the app's visual and interaction design. Hand it to a designer, or
paste it into a design tool, whole. It is self contained on purpose, so nobody needs to
read the rest of the repo to start work.

The engineering is done and working. The interface is not. What exists today is a debug
skeleton, one scrolling column of test widgets, and it needs a real design built from
nothing.

Work in **Material 3 Expressive**. Light theme is the primary design. Dark theme is
supported and switchable, and gets the same care.

---

## 1. What Betar is

Betar is a messaging and community safety app that works with no SIM, no internet, no
router and no cell tower. Phones pass messages directly to each other over Bluetooth and
Wi-Fi Direct. A message hops from phone to phone until it reaches the person it was meant
for. There is no server anywhere, no account, no login, no phone number.

The whole design serves one sentence:

> **Betar. Connect with no network.**

Where it is meant to be used:

- **Remote areas** with weak, patchy or no coverage. Hill country, islands, forest and
  river regions, high valleys.
- **During cyclones, floods and earthquakes**, when towers go down and power fails, which
  is exactly when people need to reach each other most.
- **Any time the network is down or shut off**, for whatever reason, for as long as it
  lasts.

Tone is calm public utility. This is a tool a relief worker hands to somebody in a
village. It is not a tech product, not a security product, not a lifestyle app.

## 2. The name

*Betar* (বেতার) is Bengali for wireless. Literally *be-* (without) and *tar* (wire).

It honours Jagadish Chandra Bose. In November 1895, in Calcutta, he sent radio waves
through two walls and a seated man's body to ring a bell and set off gunpowder in the
next room. That was a public demonstration of wireless transmission before Marconi's. He
worked at millimetre wavelengths a century before ordinary phones got there, and he
refused to patent any of it because he thought the work belonged to everyone.

Betar is open source under AGPL-3.0. Same idea, 130 years later.

Put the tribute on the About screen, written warmly and short. Do not overclaim. He
demonstrated wireless publicly before Marconi and gave it away unpatented. Do not write
that he invented radio, because that is genuinely contested and this project does not
make claims it cannot back.

## 3. How to frame this app, and how not to

### Never call it a chat app

In every piece of copy, store listing, onboarding screen and About page, Betar is
communication and safety infrastructure for places the network does not reach. It is not
"a messenger" and it is never described as one.

That is not spin. Alongside messaging it carries emergency alerts, a community notice
board, a help and supplies board, and offline maps. Calling it a chat app shrinks it to
its smallest part.

### But chat is tab one, the default screen, and the main surface

Messaging is what people open the app for on an ordinary day, and ordinary use is exactly
what makes the app already installed and already understood when a cyclone arrives. So:

- Chats is the first tab and the screen the app opens to, every single time.
- Chats gets the most design attention and the most polish of anything in the app.

The rule above governs words. This one governs layout. They do not conflict.

### Words to use

remote areas, when the network is down, during a cyclone, after a storm, no coverage,
dead zone, towers down, nearby, carried, travelling, works without internet.

### Words never to use

No references to governments, authorities, censorship, circumvention, bans, blocks,
protest, activism or surveillance. Not in copy, not in illustration, not in example
messages. This app is about storms, distance and broken infrastructure. An outage is an
outage and is never attributed to anyone.

Also avoid: online and offline as a binary, server, cloud, connecting, sign in, account.
Keep security jargon out of primary copy. Do not write "end to end encrypted". Write
"only the person you are writing to can read it".

### Nobody is named

There is no developer byline anywhere in the app. No personal name, no company, no
contact person. Attribution goes to the project and to its source code, which anyone can
read, build and check:

    https://github.com/betar-mesh/project-mesh

Transparency here comes from the code being open, not from a name on a page. Where a
document would normally name a company, it links the repository instead.

One line of explanation belongs in About, so the link does not confuse anyone: Betar is
built on Project Mesh, the open protocol and core that makes it work. Both live in the
same place.

## 4. Who uses it

- People in remote and rural areas, on inexpensive Android phones. Around 2 GB of RAM,
  small screens, Android 8 and up, often cracked, often at fifteen percent battery.
- A wide range of reading ability. Being able to read must not be a requirement for
  calling for help, understanding an alert or sending a message.
- Many languages. Bengali, Assamese, Hindi and Bodo first, English second. Indic scripts
  must shape correctly, with proper conjuncts and matras.
- Emergency conditions. One handed, panicking, outdoors in glare, wet hands, worried
  about battery, possibly hurt.
- Relief workers and camp coordinators as a second audience, who need density and speed.

## 5. Hard constraints

1. **No onboarding.** No signup, no account, no email, no phone number, no login. Keys
   are generated quietly in the background and never mentioned to the user. Install to
   first message should take under thirty seconds.
2. **Icons lead, not text.** Primary actions are large labelled pictograms. Voice notes
   are a first class input, equal to typing, and the microphone control is bigger than
   the keyboard control. Every icon has a spoken label.
3. **Offline is the normal state.** No screen ever waits on a network. No connectivity
   spinners. There is no "no internet" error, because that is the expected condition and
   not a failure.
4. **Delivery status has to be honest.** Messages travel when phones happen to meet, and
   that can take minutes or hours. Never show a checkmark that implies delivery is
   guaranteed. Build a four state indicator:

   | State | Means |
   |---|---|
   | Waiting | Still on your phone, nobody nearby yet |
   | Travelling | Handed to one phone |
   | Spreading | Handed to several phones |
   | Delivered | The recipient confirmed it |

   Do not borrow WhatsApp's tick language. People read ticks as a promise this system
   cannot make.
5. **Red means emergency and nothing else.** Never for errors, deletion, destructive
   actions or decoration. Form errors and warnings use amber. This is a safety rule, not
   a style preference.
6. **Never rely on colour alone.** Every state is carried by at least two of colour,
   shape, pictogram and text. Red and green colour blindness must not hide an emergency.
7. **Accessibility.** Touch targets 56dp or larger. Body text 18sp minimum. Layouts
   survive 200 percent font scaling. A high contrast sunlight mode exists for both light
   and dark, because this app gets used outdoors in glare.
8. **Cheap phones.** Restrained motion, no heavy blur, no large images, small install
   size. Honour the system reduce motion setting.

## 6. Design system

Build a full Material 3 Expressive system, and use what Expressive actually offers rather
than defaulting to plain M3.

- **Shape carries meaning.** Use the expanded shape library. Give every emergency and
  supply category its own shape as well as its own pictogram, so a category is
  recognisable by silhouette alone. That makes it readable without literacy, without
  colour, and at a glance in bad light. Use shape morphing on press for primary controls.
- **Motion.** Spring based physics on the mesh indicator and on send and receive.
  Standard motion everywhere else. Nothing decorative that costs battery.
- **Typography.** Use Expressive's heavier emphasis styles for hero state, emergency
  categories and delivery status. Restraint everywhere else.
- **Filled containers over outlines.** Expressive leans on tonal containers, and they
  also hold up better in glare than hairline borders.
- **Components.** FAB menu for new conversation actions, button groups for Have and Need
  and for board tabs, split button for send with or without location, floating toolbar
  for in thread actions, the loading indicator for looking for phones nearby.

### Colour

| Role | Hex | Notes |
|---|---|---|
| Brand blue | `#4BA3E0` | Logo, large shapes, the mesh ripple |
| Deep blue | `#12608F` | Text, icons, primary actions. Light blue alone fails contrast on a near white ground, so this carries anything small |
| Off white blue | `#EEF4F9` | Main ground. Blue tinted, never pure white |
| Lifted surface | `#F7FAFD` | Cards sitting on the ground |
| Ink | `#101A22` | Body text |
| Emergency | `#C8102E` | SOS only. Never anything else |
| Connected and verified | `#2E7D32` | Always paired with a shape or icon |
| Caution and unverified | `#9A6700` | Takes the job red normally does in Material |

Note the deliberate break from Material: the standard error role is amber here, and a
custom emergency role owns red. Document that clearly in the system so nobody undoes it
later.

Produce full tonal palettes for four themes:

1. Light, which is the primary design
2. Light high contrast, the sunlight mode
3. Dark
4. Dark high contrast

Light should feel bright and open, not clinical. Avoid stark white grounds, since cheap
LCD panels glare badly.

### Type

Noto Sans, plus Noto Sans Bengali and Noto Sans Devanagari, bundled with the app so text
renders properly on phones with poor font coverage. Set a scale with an 18sp body base.
Check conjuncts and matras in every mockup that shows Bengali or Devanagari. Do not fake
Indic text with Latin placeholders.

## 7. The logo

The mark is already designed and locked. Files are in `docs/assets/`.

| File | Use |
|---|---|
| `betar-logo.svg` | Primary mark |
| `betar-logo-compact.svg` | 24dp and below |
| `betar-logo-mono.svg` | Single colour, takes `currentColor` |

It is a solid blue field with a wire cut clean out of it, broken in the middle. The wire
is present only as its own absence, which is what the name means. The break is a real
transparent hole, so the mark sits on any ground.

Geometry, if it needs adapting: field is 108 square with a 28 corner radius, wire sits at
y 54 with stroke 14 and round caps, left bar renders from 20 to 46 and the right bar from
62 to 88. That gives a 16 unit break in the centre and 20 units of field on each side.

Two things to know:

- Below roughly 24dp, use the compact file. It has a fatter wire and a wider break. A
  closed gap is not a smaller version of this logo, it is a different logo.
- An Android adaptive icon cannot have a true hole, because the foreground layer sits on
  the background layer rather than on the wallpaper. Build the launcher icon inverted:
  background filled `#4BA3E0` edge to edge, foreground drawing the two bars in `#EEF4F9`.
  Identical to look at, backwards underneath.

## 8. Information architecture

Five destinations in the bottom bar, plus one emergency control that never goes away.

1. **Chats**, the default landing screen and the main surface
2. **Nearby**, who is around right now
3. **Board**, with Alerts, Notices and Help
4. **Map**, offline map and community pins
5. **You**, identity, language, settings, documents

Above all of it sits a **mesh ribbon**: current state as a living ripple, showing Off,
Looking, or Connected with a count, plus a small permanent mark meaning no network in
use. That ribbon is how the core promise stays visible on every screen in the app.

A red emergency button sits on every screen. It never scrolls away and it is never hidden
behind a menu.

## 9. Screens

### Onboarding

1. Language picker. Large tiles, each in its own script, no English gate.
2. Three swipeable illustrated panels, eight words or fewer each, with a read aloud
   button. Works with no internet. Your phone passes messages for others. Reach anyone
   nearby, even with no network.
3. Nickname, optional and skippable, auto generates one.
4. Permissions explained in plain language, then the system dialogs.
5. Battery exemption walkthrough, illustrated, per manufacturer. Xiaomi, Oppo, Vivo,
   realme, Samsung and OnePlus each need different steps.

### Chats, the main surface, give this the most craft

6. Conversation list, with delivery state and conversation type on each row.
7. Empty state that teaches how messages travel rather than apologising.
8. New conversation FAB menu: scan a code, join a group by name, start a private group.
9. Scan a code to add somebody.
10. In person verification. Two phones side by side showing the same short code in very
    large type plus a scannable code, and one confirm action. No cryptographic language
    anywhere on this screen.
11. One to one conversation, unverified, with an amber banner.
12. One to one conversation, verified.
13. Voice note recording. Hold to record, slide to cancel, waveform.
14. Join a group by name, with an unmissable warning that anyone who knows the name and
    passphrase can read everything in it.
15. Private group creation and adding members by code.
16. Group conversation.
17. Message long press actions in a floating toolbar. Resend, copy, delete for me.

### Nearby

18. Phones nearby. Ripple hero, one card per device with name, signal, link type and a
    trust chip.
19. Looking, and none found. An empty state that teaches, plus the relay mode toggle.
20. Device detail sheet. Message, verify in person, signal detail.

### Board

21. Alerts feed. Emergency cards with category pictogram, distance and direction if
    shared, time, acknowledgement count, and a full width acknowledge action.
22. Notices feed. Category chips, one card per notice, with a visible expiry countdown.
23. Help board. Have and Need button group, category grid, contact action.
24. Compose a notice or a help entry. Pictogram grid first, text optional, voice optional.

### Emergency, the highest stakes flow in the app

25. Category picker. Five very large tiles, each with its own shape, pictogram and word,
    and long press speaks the label. Medical, trapped, fire, danger, other.
26. Detail. Location sharing off by default, with an honest line about the tradeoff.
    Optional voice note, optional text.
27. Slide to send. A deliberate gesture, so it cannot go off in a pocket.
28. Live status. Carried by three phones nearby, one person has acknowledged. Real
    counters from the relay engine, never a fake delivered.
29. Mark resolved, or cancel.

### Map

30. Offline map with community pins.
31. No map data for this area, offering to receive it from a phone nearby.
32. Drop a pin, with the category grid.

### You

33. Identity card. Nickname, large scannable code, short verification code, copy action.
34. Language switcher.
35. Readiness checklist. Permissions, battery, Bluetooth, each with a state and a one tap
    fix. This is what somebody checks before a storm.
36. Carrying for others. "You are holding 47 messages for people around you." Frame it as
    contribution, with a storage control.
37. Appearance. Light, dark, follow system, plus the sunlight mode toggle.
38. Privacy. Panic wipe with hold to confirm, and decoy passphrase setup.
39. Documents index.

### Documents

Design these as real readable pages with real final copy. No placeholder text. Plain
language, short sentences, translatable, each with a read aloud control. Build one shared
document template: a summary card at the top in very plain words, then the detail below.

40. **Privacy policy.** This one is unusual and should be designed as a feature rather
    than buried. The honest summary is close to: we collect nothing, there is no server,
    nothing you write ever leaves the phones it travels between. Cover no account, no
    phone number, no analytics, no advertising, no crash reporting sent anywhere, what is
    stored on the device and for how long, what nearby phones can and cannot see, what
    each permission is for, children's use, and how to erase everything. Close with a
    "how to check this" card: every claim on the page is verifiable in the source, and
    the app requests no internet permission at all, which anyone can confirm.
41. **Terms of use.** Short and human. Acceptable use, no warranty, provided as is,
    community maintained, and not an emergency service. No company is named because none
    exists.
42. **Safety and limits.** The most important document here. Delivery is best effort and
    depends on other phones being nearby. There is no guarantee. This is not a
    replacement for emergency services where they exist. The security has not had an
    independent review yet. The app cannot help if no other phone is in range.
43. **Permissions explained.** One card per permission, one sentence each, with the real
    reason. Include why the app deliberately has no internet permission at all.
44. **Open source licenses.** AGPL-3.0 with a one line plain summary: anyone may use,
    study, change and share this, and must keep a changed version open too. Then the
    third party components and their licences, listed compactly.
45. **About Betar.** What it is in three sentences, the Bose tribute, the licence, the
    repository link, the line explaining Betar and Project Mesh, version and build.
    Translators and contributors credited as a group, never as named individuals.
46. **How to use this in a storm.** A short offline guide. Charge early, turn relay on,
    keep the app open in a shelter, how to send an emergency alert, how to help by
    carrying other people's messages. Illustrated, minimal text.

### Store listing, also a design deliverable

47. App icon, from the locked mark, as a full adaptive icon set.
48. Feature graphic and six screenshot frames with captions. The first line of the
    listing is the promise: connect with no network. The second line covers remote areas,
    cyclones and outages. Messaging is mentioned third, as one of several things it does.

## 10. Deliverables

1. Design system page. All four themes, type scale, shape set including the category
   shapes, spacing, motion specs, component states.
2. Every screen above at phone size, using 360dp as the reference width, in light theme.
   Plus dark and sunlight versions of the emergency flow, the Alerts feed and Chats.
3. Component specs for the four state delivery indicator, the two state trust chip, the
   mesh ribbon and the category tile, with every state shown.
4. Pictogram set for emergency, notice and supply categories, understandable with no
   text, each paired with its own shape.
5. Annotated flows for the three journeys that decide whether this app works: install to
   first message, adding and verifying somebody in person, and sending an emergency
   alert.
6. Empty, error and low battery states for every screen.
7. All document pages, with real final copy.
8. One screen shown in Bengali and in Hindi, to prove the layout survives longer strings
   and complex script shaping.

## 11. How to work

Ask about anything ambiguous before starting. Then propose the design system first, which
means the palette across all four themes, the type scale, the shape language and the mesh
ribbon concept, and get that agreed before producing the full screen set.
