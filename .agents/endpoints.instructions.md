# API Endpoints Reference

All endpoints are defined and documented in the OpenAPI specification file.

**Primary Source of Truth:** [openapi.yml](../docs/openapi.yml)

This document serves as a pointer to the comprehensive API documentation.

## DTO

- Export a DTO to TypeScript with `#[ts(export)]` alone, never `export_to`: ts-rs names the file after the TS type
  name, which already follows `#[serde(rename = "X")]` (`CardOfferResponse` → `CardOffer.ts`)
