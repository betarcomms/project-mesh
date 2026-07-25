# Compliance

> This is not legal advice. It covers spectrum rules for the LoRa radio backbone, the software
> licence, and the warranty position. Verify current rules before deploying hardware.

## LoRa spectrum compliance (India)

Betar's long range backbone uses LoRa radio hardware, described in `HARDWARE-LORA.md`. In India
this operates in the IN865 band, 865 to 868 MHz, licence free for low power short range devices
under GSR 853(E), 10 December 2021.

Two device classes apply:

- **Non-specific short range devices**, the class Betar's user and relay nodes fall into: 25 mW
  e.r.p., 1% duty cycle, FHSS, maximum occupied bandwidth 50 kHz for 58 or more hop channels. The
  duty cycle applies to the whole transmission, not per hop channel.
- **Tracking, tracing, and data-acquisition devices**, a higher power class: up to 500 mW e.r.p.
  with Adaptive Power Control required, duty cycle 10% for network access points or 2.5%
  otherwise, 200 kHz bandwidth.

No licence is needed for compliant devices, but only on a non-interference, non-protected,
shared, non-exclusive basis, and equipment must be type-approved. IN865 is different from EU868
and US915: hardware and firmware must be configured for the Indian band, and using a EU or US
band plan in India is not compliant.

Type approval and spectrum compliance for any LoRa deployment is the deployer's responsibility,
not something this project certifies on a deployer's behalf.

## Licence

Project Mesh, the protocol and the Rust core, and Betar, the app built on it, are released under
the GNU Affero General Public License, version 3.0 (AGPL-3.0). Full text in `LICENSE`.

AGPL-3.0 is copyleft: anyone can use, study, modify, and redistribute the code, and anyone who
distributes a modified version, or runs one as a network service, must release the source for
that version under AGPL-3.0 too. This keeps improvements to the project open and closes the
network-service loophole that a plain GPL leaves in place, so a closed fork cannot quietly offer
the same code as a service without sharing changes back.

## No warranty, no liability

The software is provided as is, without warranty of any kind, express or implied, including
fitness for a particular purpose and non-infringement. To the extent permitted by law, no
contributor to this project is liable for any claim, damages, or other liability arising from
the software or its use. Full terms in `LICENSE`.
