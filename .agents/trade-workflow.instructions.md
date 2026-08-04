# Trade Workflow

## Overview

Card-for-card trade between exactly two users. Value differences are settled in cash outside the platform. Ratings are
optional and mutual.

---

## Actors

| Actor              | Role                                               |
| ------------------ | -------------------------------------------------- |
| **Initiator (A)**  | Finds a card and requests the trade                |
| **Respondent (B)** | Owner of the requested card, makes a counter-offer |

---

## Full Flow

### 1. Discovery

- User _A_ searches for a card by name.
- _A_ finds a card owned by user B.

### 2. Trade Request (status: `PENDING`)

- _A_ sends a trade request for B's card.
- No card from _A_ is proposed yet — _A_ is simply saying "I want this card."
- _B_ receives a notification: **"New trade request received."**

### 3. Counter-Proposal

- _B_ browses _A_'s collection and selects zero or more cards they're interested in, in exchange.
- The interface computes the value difference and displays the cash amount owed by one party to the other.
- _B_ submits the counter-proposal. The trade then lists:
  - Cards offered by _B_ → to _A_
  - Cards offered by _A_ → to _B_ (selected by B, or none)
  - Cash delta (informational only — settled outside the platform)
- _A_ receives a notification: **"_B_ made a counter-proposal."**

### 4. Negotiation (`PENDING`)

- Either party can modify the trade (add/remove cards, adjust) at any time **while the status is
  `PENDING`**.
- Each modification triggers a notification to the other party.

### 5. First Acceptance (status: `ONE_ACCEPTED`)

- One party clicks "Accept."
- A modal warns: **"Once accepted, the trade will be locked. If the other party modifies the trade, it will go back to
  pending and will need to be accepted again."**
- On confirmation:
  - Status → `ONE_ACCEPTED`.
  - All cards on both sides of the trade are **reserved** (visible in each user's collection).
  - Any other `PENDING` or `ONE_ACCEPTED` trades involving these same cards are automatically **abandoned**.
  - The affected parties receive a notification: **"A trade involving one of your reserved cards was finalized by
    another trade. Your trade has been canceled."**
  - The other party receives a notification: **"Your partner accepted. Accept in turn or modify the trade to restart
    the negotiation."**

### 5b. Modification After Acceptance (back to `PENDING`)

- If either party modifies the trade while the status is `ONE_ACCEPTED`:
  - Status → `PENDING`.
  - Reserved cards are **released**.
  - Both previous acceptances are canceled — both parties will need to accept again.
  - The other party receives a notification: **"The trade was modified. It is back in negotiation."**

### 6. Full Acceptance (status: `FULLY_ACCEPTED`)

- The second party accepts (same warning modal).
- Status → `FULLY_ACCEPTED`.
- Both parties receive a notification: **"Both parties have accepted. Proceed with the physical exchange."**

### 7. Physical Exchange

- The cards (and cash, if applicable) are exchanged in person, outside the platform.

### 8. Trade Confirmation (status: `COMPLETED`)

- Each party confirms "Trade completed" in the app.
- Status → `COMPLETED` once **both** have confirmed.
- Both parties receive a notification: **"Trade confirmed by both parties."**

### 9. Rating (optional, status: `CLOSED`)

- Each party can rate the other from zero to five stars.
- Rating is optional — skipping it does not block closing.
- Once both have rated (or skipped), status → `CLOSED`.
- The trade rating is the average of both parties' ratings; if only one party rated the trade, that single rating is used.
- Each party has an overall rating, computed as the average of all ratings they have received across their past trades.

---

## Status Reference

```
PENDING ◄─────────────────────────────────────────────────────┐
  │  (one party accepts)                                      │
  ▼                                                           │
ONE_ACCEPTED ───(modification by either party)────────────────┘
  │  (the other party accepts)
  ▼
FULLY_ACCEPTED
  │  (both confirm the physical exchange)
  ▼
COMPLETED
  │  (both rate or skip rating)
  ▼
CLOSED

ABANDONED  ← reachable from any status before COMPLETED
```

| Status           | Description                                                                 |
| ---------------- | --------------------------------------------------------------------------- |
| `PENDING`        | Open to modification by both parties                                        |
| `ONE_ACCEPTED`   | One party has accepted; cards reserved; modifiable (goes back to `PENDING`) |
| `FULLY_ACCEPTED` | Both have accepted; awaiting the physical exchange                          |
| `COMPLETED`      | Physical exchange confirmed by both parties                                 |
| `CLOSED`         | Trade finished; ratings submitted or skipped                                |
| `ABANDONED`      | Trade cancelled by either party at any time before `COMPLETED`              |

---

## Card Reservation Rules

- Cards are reserved at the `ONE_ACCEPTED` step.
- A reserved card is shown as such in its owner's collection (badge or indicator).
- If the same card appears in several simultaneous trades:
  - The **first trade to reach `ONE_ACCEPTED`** reserves the card.
  - All other trades involving that card are automatically `ABANDONED`.
  - All affected parties are notified immediately.

---

## Modification Rules

| Who can modify | When                              | Effect on status                       |
| -------------- | --------------------------------- | -------------------------------------- |
| Either party   | Status `PENDING`                  | Stays `PENDING`                        |
| Either party   | Status `ONE_ACCEPTED`             | Goes back to `PENDING`, cards released |
| No one         | Status `FULLY_ACCEPTED` or beyond | Not possible                           |

Any modification notifies the other party.

---

## Abandonment Rules

- Either party can abandon the trade at any time before `COMPLETED`.
- On abandonment:
  - Status → `ABANDONED`.
  - Reserved cards (if any) are released.
  - The other party receives a notification: **"The trade was abandoned by your partner."**
  - The abandoning party can be rated by the other party (optional, 0–5 stars).

---

## Notification Summary

| Trigger                                           | Recipient(s)         |
| ------------------------------------------------- | -------------------- |
| Trade request sent                                | _B_                  |
| Counter-proposal submitted                        | _A_                  |
| Trade modified (status `PENDING`)                 | The other party      |
| First acceptance (`ONE_ACCEPTED`)                 | The other party      |
| Modification after acceptance (back to `PENDING`) | The other party      |
| Other trades invalidated                          | All affected parties |
| Both have accepted (`FULLY_ACCEPTED`)             | Both                 |
| Physical exchange confirmed by both (`COMPLETED`) | Both                 |
| Trade abandoned                                   | The other party      |

---

## Out of Scope (MVP)

- Online payment — the cash delta is informational only.
- Trade expiration — trades remain active indefinitely until accepted or abandoned.
- Trade history — planned post-MVP.
- Built-in messaging — may be added if deemed necessary.
