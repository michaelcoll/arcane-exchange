# Clerk Bearer Token Authentication

## Overview

Authentication based on **Clerk** using JWTs validated by the backend.

### Environment Variables

Add to `.env`:

```bash
CLERK_FRONTEND_API_URL=https://musical-pup-67.clerk.accounts.dev
```

### Protected vs Public Endpoints

Most endpoints require a bearer token. The only public (no-auth) routers are `maintenance` (`/maintenance/*`) and
`autocomplete` (`GET /autocomplete/user`). Everything else (`card`, `collection`, `search`, `trade`, `user`) uses
the `AuthenticatedUser` extractor. See [openapi.yml](../doc/openapi.yml) (or
[endpoints.instructions.md](endpoints.instructions.md)) for the full, authoritative endpoint list.

## Usage Flow

1. **Frontend (Nuxt/Vue, `frontend-vue/`)**: `useApi()` (`app/composables/useApi.ts`) obtains the token via Clerk's
   `useAuth().getToken()` and includes it in the request headers (`Authorization: Bearer <token>`).
2. **Backend (Rust)**: The `AuthenticatedUser` extractor (`infrastructure/adapter_in/auth_extractor.rs`) delegates to
   `ClerkAuthService::validate_token` (`application/service/auth_service.rs`), which validates the JWT using Clerk's
   public keys (JWKS):
   - Verifies the signature, issuer (`CLERK_FRONTEND_API_URL`), and expiration.
   - Returns HTTP 401 on failure.

## User Model

The Clerk JWT only carries `sub`, `username` and `image_url` claims — there is no `email` or `name` claim. The
domain `User` (`domain/user.rs`) built from it has:

- `id` (`UserId`): the `sub` claim (primary key used to isolate data).
- `name`: always `None` — the Clerk JWT does not include it.
- `username`: from the `username` claim, `None` if not set on the Clerk account.
- `avatar_url`: from the `image_url` claim.

**To add a new protected endpoint, use the `AuthenticatedUser` extractor in your handler function signature.**
