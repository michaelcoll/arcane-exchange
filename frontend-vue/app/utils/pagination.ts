// Mirrors the per-endpoint offset limits enforced by the backend (`*_MAX_OFFSET` constants
// next to each `application::service`, see `src/ccpt/application/service/*.rs`). The API
// rejects `page * page_size` beyond these limits with an HTTP 400 — these values let the UI
// stop paginating before hitting that error instead of firing a request bound to fail.
//
// Only the endpoints the front actually paginates past page 0 need an entry here — card offers
// (`/card/offers`) never do (the detail modal always requests a single fixed page), so there is
// no `CARD_OFFERS_MAX_OFFSET` to keep in sync.
export const COLLECTION_MAX_OFFSET = 10_000;
export const SEARCH_MAX_OFFSET = 10_000;
export const TRADES_MAX_OFFSET = 2_000;

export const canLoadPage = (page: number, pageSize: number, maxOffset: number) =>
  page * pageSize <= maxOffset;
