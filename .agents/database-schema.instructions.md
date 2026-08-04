# Database Schema & Data Flow Guide

This document details the database structure based on migrations and enhances it with context from the application's
service layer, explaining _how_ each table is used in the collection tracking system.

## Core Tables (Transactional Data)

### `set_name`

- **Role:** Stores metadata for each card set. This is a static reference table.
- **Colonnes:**
  - `set_code` (PK): Unique identifier for the set.
  - `name`: Full name of the set.

### `card`

- **Role:** Defines a unique card template within the game/set. This is the base entity for any collection item.
- **Colonnes:**
  - `set_code` (FK): Links to `set_name`.
  - `collector_number`, `language_code`, `foil`: Composite key components defining the specific card version.
  - `name`, `rarity`: Descriptive attributes of the card.
  - `scryfall_id`, `cardmarket_id`: External identifiers used for integration.
- **Application Flow:** Managed by `CardRepositoryAdapter`. All collection management (adding/updating a card) flows
  through this adapter, which uses the composite key to ensure uniqueness.

### `card_quantity`

- **Role:** Tracks a specific user's collection instance of a card. This is the transactional record of ownership.
- **Colonnes:**
  - `set_code`, `collector_number`, `language_code`, `foil`: Composite FK referencing the base card in `card`.
  - `user_id`: Identifies the owner.
  - `quantity`, `purchase_price`: Transactional data specific to this user's acquisition.
- **Application Flow:** Updated atomically by `CardRepositoryAdapter` upon collection edits, using an `ON CONFLICT`
  clause to handle quantity adjustments rather than creating duplicate entries.

### `cardmarket_price`

- **Role:** The append-only ledger for all raw price data ingested from external sources (CardMarket). It is the
  historical record.
- **Colonnes:**
  - `id_produit`, `date`: Composite key defining a specific price snapshot.
  - `low`, `trend`, `avg`, etc.: Raw pricing metrics for the date.
  - `low_foil`, `trend_foil`, `avg_foil`, etc.: Specific metrics for foil versions.
- **Application Flow:** Managed by `CardMarketPriceRepositoryAdapter`. Ingestion is batched and transactionalized using
  a chunking mechanism (`CHUNK_SIZE = 1000`) to efficiently handle high-volume updates.

## Derived/Read Models (Aggregated Data)

### `mv_card_prices` (Materialized View)

- **Role:** Provides a fast, read-optimized snapshot of the most recent market pricing for quick display. It is derived
  from `cardmarket_price`.
- **Colonnes:** Contains the most relevant pricing metrics joined with card details.
- **Application Flow:** This view must be explicitly refreshed (`REFRESH MATERIALIZED VIEW CONCURRENTLY`) by the
  application to ensure it reflects the latest ingested data. It is used for read operations, not transactional writes.

---

_This guide integrates the SQL structure with the application's data persistence layer responsibilities._
