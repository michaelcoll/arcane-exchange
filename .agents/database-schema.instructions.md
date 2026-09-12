# Database Schema Guide

**Source of truth for the schema: [`docs/db.md`](../docs/db.md)** — generated ERD (tables, columns, types, PK/FK,
indexes, constraints). Read it for column names and relations; this file only documents what the ERD cannot express.

## Invariants

- **Card identity is `(set_code, collector_number, language_code)`** — never key a card by `scryfall_id` or
  `cardmarket_id`. `foil` belongs to the _copy_, so it is part of the unique key of `collection_entry` /
  `trade_card`, never of their foreign key to `card`.
- **All prices are integers in cents**, `_foil` variants included.
- **`cardmarket_price` is append-only**: one row per product per day, never updated.
- **The materialized views are stale by design and read-only.** `mv_card_prices` is collection-gated (one row per
  owned copy); `mv_last_cardmarket_prices` is keyed `(set_code, collector_number, foil)` without `language_code`,
  because a Cardmarket price doesn't depend on the language.
- **Refresh order is load-bearing**: `mv_last_cardmarket_prices` must be refreshed _before_ `mv_card_prices`, which
  reads it. Get it wrong and `mv_card_prices` silently serves prices one cycle stale — no error either way.
  `CardPricesViewRepositoryAdapter::refresh()` is the single place doing both in that order; every flow that
  imports prices or cards must go through it.
- `REFRESH MATERIALIZED VIEW CONCURRENTLY` requires a unique index — keep `mv_card_prices_unique` if the view
  changes.
- **A trade reads `mv_last_cardmarket_prices` directly** (joined on `trade_card`, whose own `foil` supplies the
  finish) rather than the collection-gated `mv_card_prices`, so a trade still shows a card's price after its owner
  removes the card from their collection.
- **Card reservation is derived, not stored**: a card is reserved when it appears in `trade_card` of a non-terminal
  trade.
- **`v_tradable_entry` is the only source of what a player actually offers**, derived from `users.visibility`,
  `trading_binders` and `collection_rarity_filters`. It deducts `kept_copies` **per `collection_entry` row (per
  binder)**, not once per aggregated card total — this must stay numerically identical to the "Proposés" counter of
  `/collection/visibility/rarities`, or a card split across several checked binders offers more copies for trade
  than the profile screen announces.

## Changing the schema

1. Add `migrations/NNNN_description.sql` (4-digit sequence, forward-only — no down migrations). Applied at startup.
2. If a view's shape changes, drop and recreate it in the same migration — dropping it drops its indexes too, so
   recreate them (`mv_card_prices_unique` in particular).
3. Run `mise run rebuild-db-doc` (regenerates `docs/db.md`) and `mise run sqlx-prepare`; both are covered by
   `mise run checks`.
