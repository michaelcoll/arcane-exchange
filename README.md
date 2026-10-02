![](docs/images/04-split-diagonal.svg "Logo")

[![codecov](https://codecov.io/gh/michaelcoll/arcane-exchange/graph/badge.svg?token=b2Wlmg2WX3)](https://codecov.io/gh/michaelcoll/arcane-exchange)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Track what your Magic: The Gathering collection is worth, find the cards you're missing in other players'
collections, and trade them — card for card.

## Features

- **Your collection, valued** — import a Manabox export and see the total value, the 30-day trend and a price history
  for every card.
- **Real market prices** — daily prices from Cardmarket, card data from Scryfall, playability signals from EDHREC.
- **Find who owns a card** — search by card name, by decklist, or browse a specific player's collection.
- **Trade in two clicks** — request a card, get a counter-offer, negotiate. The app computes the value difference; the
  cash delta is settled between players, off-platform.

- **On your iPhone too** — a native iOS app, with home-screen widgets for your collection and your ongoing trades.

**Stack** — Rust (Axum, SQLx) · Nuxt 4 / Vue 3 / Tailwind · SwiftUI · PostgreSQL 18 · Clerk for authentication.

## Quick start (Docker Compose)

```bash
cp .env.example .env   # then fill in your Clerk keys
docker compose up -d
```

- App: <http://localhost:9797>
- API: <http://localhost:8080>

`docker-compose.yml` stores the database and card images under `/mnt/ssd/…`: adjust those volume paths to your
machine first.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) to set up a development environment and open a pull request.

## Configuration

Copy `.env.example` to `.env` and fill it in — the Clerk keys are the only values you must provide to run the app.
Everything else (database URL, ports, external API endpoints) has a working default in `mise.toml` for local
development, and in `docker-compose.yml` for Compose.

Authentication is handled by [Clerk](https://clerk.com/): create an instance, then set `CLERK_FRONTEND_API_URL`
(used by the backend to validate JWTs) along with the publishable and secret keys used by the frontend.
