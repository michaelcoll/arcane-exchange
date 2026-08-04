# Clerk Bearer Token Authentication

## Overview

Authentication based on **Clerk** using JWTs validated by the backend.

### Environment Variables

Add to `.env`:

```bash
CLERK_FRONTEND_API_URL=https://musical-pup-67.clerk.accounts.dev
```

### Protected Endpoints (Token Required)

- `GET /collection`: User's paginated collection.
- `POST /collection/import`: Card import (ManaBox CSV).
- `POST /card/card-info`: Information about a card.

### Public Endpoints

- `GET /maintenance/stats`: Global statistics (no authentication).
- `POST /maintenance/trigger-price-update`: Manual trigger of the price update.

## Usage Flow

1. **Frontend (Angular)**: The `AuthService` obtains the token via `clerk.session.getToken()` and includes it in the
   request headers (`Authorization: Bearer <token>`).
2. **Backend (Rust)**: The `AuthenticatedUser` extractor handles JWT validation using Clerk's public keys (JWKS):
   - Verifies the signature, issuer (`https://musical-pup-67.clerk.accounts.dev`), and expiration.
   - Returns HTTP 401 on failure.

## User Model

Each user is identified by:

- `user.id`: The "sub" claim from the Clerk JWT (primary key used to isolate data).
- `user.email`: The user's email address.

**To add a new protected endpoint, use the `AuthenticatedUser` extractor in your handler function signature.**
