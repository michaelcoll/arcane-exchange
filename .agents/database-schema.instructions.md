# Database Schema Guide

**Source of truth for the schema: [`doc/db.md`](../doc/db.md)** — generated ERD (tables, columns, types, PK/FK,
indexes, constraints). Read it when you need column names or relations; this file only documents what the ERD cannot
express.

## Table roles & ownership

Each table is owned by one adapter in `src/ae/infrastructure/adapter_out/repository/`.

| Table                      | Role                                                    | Adapter                                       |
| -------------------------- | ------------------------------------------------------- | --------------------------------------------- |
| `set_name`                 | Static reference: card sets                             | `set_names_repository_adapter`                |
| `card`                     | Card template (game data, no ownership)                 | `card_repository_adapter`                     |
| `collection_entry`         | A user's ownership of a card (quantity, purchase price) | `card_repository_adapter`                     |
| `cardmarket_price`         | Append-only ledger of daily CardMarket prices           | `cardmarket_price_repository_adapter`         |
| `collection_price_history` | Daily valuation of a user's collection                  | `collection_price_history_repository_adapter` |
| `users`                    | Local mirror of Clerk users (id, username)              | `user_repository_adapter`                     |
| `trade`, `trade_card`      | Trades and the cards engaged in them                    | `trade_repository_adapter`                    |
| `mv_card_prices`           | Materialized view: collection joined with latest prices | `card_prices_view_repository_adapter`         |

`v_tradable_entry` (plain, non-materialized view) is an exception to "one table, one adapter": it derives the
quantity of each card a user actually offers to trade (from `users.visibility`, `trading_binders` and
`collection_rarity_filters`) and is read directly, in SQL, by three adapters —
`card_prices_view_repository_adapter` (`/card/offers`, public mode of `/search/card`), `user_repository_adapter`
(`/autocomplete/user`) and `trade_repository_adapter` (validating cards added to a trade). It owns no table and
is never written to.

## Invariants

- **Card identity** is the composite key `(set_code, collector_number, language_code, foil)`. It propagates to
  `collection_entry`, `trade_card` and `mv_card_prices`. Never key a card by `scryfall_id` or `cardmarket_id`.
- **All prices are integers in cents** (`purchase_price`, `low`, `trend`, `avg`, and their `_foil` variants).
- **Upserts, not duplicates**: `card`, `collection_entry`, `users`, `set_name` and `trade_card` writes use
  `ON CONFLICT ... DO UPDATE` on their natural key.
- **`cardmarket_price` is append-only** and ingested in chunks (`CHUNK_SIZE = 1000`) per transaction.
- **`mv_card_prices` is stale by design**: refreshed explicitly with `REFRESH MATERIALIZED VIEW CONCURRENTLY`
  after every price import and card/collection import (`import_price_service`, `import_card_service`, and the
  CardMarket/Gatherer update workers). Read-only — never write to it.
  `CONCURRENTLY` requires the unique index `mv_card_prices_unique`; keep it if the view changes.
- **Trade card reservation** is derived, not stored: a card is reserved when it appears in `trade_card` of a
  non-terminal trade (see [trade-workflow.instructions.md](trade-workflow.instructions.md)).
- **`v_tradable_entry` deducts `kept_copies` per `collection_entry` row (per binder), not once per aggregated
  card total.** This must stay numerically identical to `collection_rarity_filters_repository_adapter`'s
  "Proposés" counter (`/collection/visibility/rarities`), which does the same per-row deduction — a card split
  across several checked binders must not offer more copies for trade than the profile screen shows as
  proposed.

## Changing the schema

1. Add `migrations/NNNN_description.sql` (4-digit sequence, forward-only — no down migrations). Applied at startup.
2. If the view's shape changes, drop and recreate `mv_card_prices` in the same migration — dropping it also drops its
   indexes, so recreate `mv_card_prices_unique` (and any other index the view had).
3. Run `mise run rebuild-db-doc` to regenerate `doc/db.md`, and `mise run sqlx-prepare` to refresh the SQLx metadata
   (both are covered by `mise run checks`).
