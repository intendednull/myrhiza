**Date:** 2026-05-22
**Status:** active
**Subject:** Signal Foundation — governance, funding model, key people, contributor process.

# Signal Foundation — governance and funding

## Organization

**Signal Messenger, LLC** is a wholly-owned subsidiary of the **Signal
Technology Foundation** (commonly "Signal Foundation"), a 501(c)(3)
nonprofit registered in the US (Delaware).

Founded: **2018-02-21** by Moxie Marlinspike and Brian Acton. Replaced
Open Whisper Systems (which had been a Delaware C-corp, Quiet Riddle
Ventures LLC) as the development organization.

## Funding model

**Donations + Acton loan + service fees.** Per the 2023-11 "Signal is
expensive" blog post:

| Source | Magnitude |
|---|---|
| Brian Acton initial loan (2018) | $50M |
| Annual operating cost (2025 projected) | $50M |
| Annual infrastructure (2023) | $14M (storage $1.3M + servers $2.9M + SMS-registration $6M + bandwidth $2.8M + other $0.7M) |
| Annual personnel (2023) | $19M (~50 FTE) |
| Revenue model | Donations only — no advertising, no data sales, no premium tier |

The Acton loan is structured as a loan, not a grant — Signal Foundation
owes the money back. The terms have not been made fully public, but
Marlinspike has stated in interviews that the loan is "patient capital"
without aggressive repayment pressure.

**Sustainability concern.** Signal Foundation's stated goal is to
transition fully to donation-based funding before the Acton loan
requires repayment. Per the 2023-11 blog post, donation growth is the
critical metric — Signal does not break out donation receipts publicly,
but the budget gap between current donations and full operating cost
remains material.

A small premium-feature tier ("Signal Plus," extra storage, etc.) has
been rumored but not deployed as of 2026-05-22.

## Key people

### Moxie Marlinspike (founder, ex-CEO)

- Founded Open Whisper Systems (January 2013).
- Co-author of X3DH (2016), Double Ratchet (2016), Sealed Sender design.
- CEO of Signal Foundation 2018-02-21 to 2022-01-10.
- Remains on the Signal Foundation board.
- Departure stated reason (per Marlinspike's blog post): wanting fresh
  leadership for the next decade.

### Trevor Perrin (protocol designer)

- Co-author of X3DH, Double Ratchet, Sealed Sender.
- Editor of all three Signal Protocol specifications.
- Independent cryptographer; not formally a Signal Foundation employee
  but functionally the lead protocol designer.
- Also a primary contributor to the MLS specification (RFC 9420) and the
  Noise Protocol Framework.

### Rolfe Schmidt (cryptographer)

- Co-author of Double Ratchet spec revision 3+ (added in 2017-2018
  range).
- Co-author of PQXDH spec.
- Signal Foundation employee; serves as the public-facing cryptography
  lead post-Marlinspike.

### Ehren Kret

- Co-author of PQXDH spec (with Schmidt).
- Signal Foundation engineer.

### Brian Acton (board)

- Co-founder of WhatsApp.
- Departed WhatsApp/Facebook in September 2017, citing data-privacy
  disagreements (notably tweeted "It is time. #deletefacebook" on
  2018-03-20).
- Co-founded Signal Foundation 2018-02-21 with $50M loan.
- Served as interim CEO between Marlinspike's departure (2022-01-10)
  and Whittaker's appointment (2022-09).
- Currently Executive Chairman of the Signal Foundation board.

### Meredith Whittaker (President)

- Appointed September 2022.
- Background: AI ethics researcher at Google (2006-2019); co-founder of
  AI Now Institute at NYU.
- Vocal critic of surveillance capitalism, advertising-funded business
  models, and AI deployment without oversight.
- Holds the **President** title; the CEO role has not been formally
  filled since Marlinspike's departure. The board explicitly chose not
  to refill the CEO role.
- Public-facing voice for Signal Foundation post-2022; appears at policy
  conferences, congressional testimony, etc.

### Headcount

~50 FTE per the 2023-11 blog post. Lean by tech-industry standards.

## Board structure

Per Signal Foundation's IRS filings (public via Form 990):

- Brian Acton — Executive Chairman
- Moxie Marlinspike — Director (post-CEO departure)
- Katherine Maher — Director (former CEO of Wikimedia Foundation;
  joined the board around 2019)
- Meredith Whittaker — President (and Director ex officio in many
  meeting structures)

Exact board composition has had some turnover; refer to current Form 990
filings for the live roster.

## Contributor License Agreement

External contributions to libsignal, Signal-Android, Signal-iOS,
Signal-Desktop, and Signal-Server require signing **Signal's CLA**. The
CLA grants Signal Foundation broad relicensing rights over contributed
code.

This is unusual for open-source projects in the cryptography space.
Comparable projects:

- OpenMLS: no CLA, contributors retain copyright (Apache-2.0 + MIT
  dual-license).
- RustCrypto: no CLA.
- BoringSSL: Google CLA (similar shape to Signal's).
- vodozemac (Matrix): no CLA.

Practical effect: Signal Foundation can relicense future versions of
libsignal without contributor consent. If Signal Foundation chose to
move from AGPL to a proprietary license, they could. This is
unlikely in practice (the AGPL is part of the brand) but is a
governance fact worth knowing.

The CLA also has a chilling effect on cryptographer contribution —
several academic-cryptographer-equivalent contributors to Signal's
specifications publish their work in spec form (via Signal's
publication channels) rather than as code patches.

## Decision-making for protocol changes

Protocol-spec changes go through:

1. Internal design + prototype at Signal Foundation.
2. Spec document drafted and published at signal.org/docs.
3. Discussion on the [whispersystems-discuss] mailing list (now
   defunct; replaced by GitHub Issues + a community Matrix room).
4. No formal RFC process or external standards body involvement.

Compare to MLS, which went through the IETF's MLS Working Group and
produced RFCs 9420 (protocol) + 9750 (architecture). Signal's protocol
*could* have been standardized at IETF; explicitly was not. Per
Marlinspike (in various interviews), standardization would constrain
iteration speed.

This is the same argument as "the ecosystem is moving" applied at the
protocol-design layer.

## Implications for Myrhiza

- **Single-operator governance is the load-bearing assumption behind
  Signal's iteration speed.** Marlinspike's federation argument is
  *also* a governance argument: no consensus needed = fast iteration.
  Myrhiza as a P2P runtime cannot replicate this — protocol changes
  in Myrhiza apply across many independent operators.
- **The CLA pattern is a warning.** A project that wants community
  contribution + retains relicensing rights is taking a real
  community trust hit. Myrhiza should default to no CLA unless there
  is a specific reason.
- **The Acton-loan-funded nonprofit is a unique funding model that
  has worked.** Few open-source projects have a single-donor $50M
  patient capital injection. Don't assume Myrhiza can replicate the
  funding model.
- **The absence of a CEO post-2022 is unusual.** Whittaker holds
  effective executive authority as President; the board chose not to
  call the role "CEO." This is governance signaling — the position
  rejects the standard tech-company CEO mythology. Worth noting for
  Myrhiza governance discussions.
- **Standards-body engagement vs go-it-alone protocol design has
  real costs.** Signal's iteration-speed argument is real, but the
  cost is: third-party implementations of Signal Protocol (Matrix
  olm/vodozemac, WhatsApp, etc.) drift in subtle ways, and there is
  no formal compliance test suite. MLS's IETF process produced RFC
  9420 + an MLS WG test vector suite. If Myrhiza wants
  interoperability with implementations it doesn't control, it
  should engage a standards body. If it doesn't, the iteration-
  speed benefit is real.

## Sources

- "Signal is expensive" (annual costs): <https://signal.org/blog/signal-is-expensive/> (2023-11-16)
- Signal Foundation founding announcement: <https://signal.org/blog/signal-foundation/> (2018-02-21)
- Marlinspike's CEO departure: <https://moxie.org/2022/01/10/moving-on.html> (2022-01-10)
- "The Ecosystem is Moving" (governance / federation argument): <https://signal.org/blog/the-ecosystem-is-moving/> (2016-05-10)
- Signal Foundation Form 990 filings: search Pro Publica Nonprofit Explorer for "Signal Technology Foundation"
- Wikipedia: Signal Messenger — <https://en.wikipedia.org/wiki/Signal_Messenger>
- Comparator: `prior-art/mls/governance.md` (IETF WG governance contrast)
