/**
 * The one shape an API failure reaches a caller in (ADR 0018). Never test its type (`instanceof`):
 * read any caught value with `readApiError`, which is the only place allowed to look at the raw
 * error response.
 */
export interface ApiError extends Error {
  /** Stable code of the API, as the backend emits it. Absent without a response body carrying one. */
  readonly code?: string;
  /** HTTP status. Absent when the API did not answer (network failure). */
  readonly status?: number;
  /** French message, ready to show to the player. */
  readonly message: string;
  /** The API's technical message, in English: for diagnosis only, never shown. */
  readonly technicalMessage: string;
}

const GENERIC_MESSAGE = 'Une erreur est survenue.';
const UNKNOWN_TECHNICAL_MESSAGE = 'Unknown error';

/**
 * French message of each stable code the API returns: the codes are copied from
 * `FunctionalError::code` and `AppError::code` in the backend, and renaming one there breaks this
 * table silently.
 */
const MESSAGES: Record<string, string> = {
  empty_file: 'Le fichier est vide.',
  malformed_csv: "Ce fichier n'est pas un CSV lisible.",
  binder_export:
    'Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox.',
  unrecognized_format: "Ce fichier n'est pas un export collection ManaBox reconnu.",
  no_valid_line: "Aucune ligne du fichier n'a pu être lue.",
  import_already_running: 'Un import est déjà en cours.',
  import_not_found: "Cet import n'existe pas.",
  self_trade: 'Tu ne peux pas ouvrir un échange avec toi-même.',
  trade_not_modifiable: "Cet échange n'est plus modifiable.",
  trade_not_found: "Cet échange n'existe pas.",
  trade_access_denied: "Tu n'as pas accès à cet échange.",
  trade_not_acceptable: 'Cet échange ne peut plus être accepté.',
  trade_already_accepted: 'Tu as déjà accepté cet échange.',
  trade_already_finalized: 'Cet échange est terminé, il ne peut plus être abandonné.',
  trade_not_fully_accepted: "Les deux joueurs doivent accepter l'échange avant de le confirmer.",
  trade_already_confirmed: 'Tu as déjà confirmé cet échange.',
  trade_not_completed: "Tu pourras noter cet échange une fois qu'il sera terminé.",
  trade_already_rated: 'Tu as déjà noté cet échange.',
  trade_concurrently_modified:
    "L'échange vient d'être modifié. Recharge la page avant de recommencer.",
  trade_empty: "Ajoute au moins une carte avant d'accepter cet échange.",
  trade_card_not_found: "Cette carte ne fait plus partie de l'échange.",
  card_already_reserved: 'Cette carte est déjà réservée dans un autre échange.',
  binder_not_found: "Ce binder n'existe plus dans ta collection.",
  user_not_found: 'Ce joueur est introuvable.',
  // Also what adding a card to a trade answers when its owner no longer offers it.
  card_not_found: "Cette carte est introuvable ou n'est plus disponible.",
  set_not_found: 'Ce set est introuvable.',
  unauthorized: 'Ta session a expiré, reconnecte-toi.',
  internal: 'Erreur technique, réessaie plus tard.',
};

/**
 * French message of a bare code, for a failure that does not arrive as an error: a card import
 * that failed in the background is persisted with its code. An unknown or missing code gets
 * `fallback`, the generic message unless the caller has its own.
 */
export const apiErrorMessage = (
  code: string | null | undefined,
  fallback: string = GENERIC_MESSAGE,
): string => (code && Object.hasOwn(MESSAGES, code) ? MESSAGES[code] : undefined) ?? fallback;

type Fields = Record<string, unknown>;

const fields = (value: unknown): Fields =>
  typeof value === 'object' && value !== null ? (value as Fields) : {};

const text = (value: unknown): string | undefined =>
  typeof value === 'string' && value !== '' ? value : undefined;

const httpStatus = (value: unknown): number | undefined =>
  typeof value === 'number' && value > 0 ? value : undefined;

/**
 * Turns any caught value into an `ApiError`: a raw `$fetch` error, the error `useAsyncData`
 * exposes, an `ApiError` already read, or anything else. An unknown or missing code gets the
 * generic message, never the technical one.
 *
 * `useAsyncData` rewraps what its handler rejects (`createError`), keeping only `message`,
 * `statusCode` and `data` — the same three the SSR payload serializes. So the result carries the
 * status and the response body under those names too, and reading it again, on either side of
 * that trip, gives the same error. One limit: `useAsyncData` gives a failure without a response
 * the status 500.
 */
export const readApiError = (value: unknown): ApiError => {
  const raw = fields(value);
  const body = fields(raw.data);

  const code = text(body.code);
  // The only place that reads the status under its two forms.
  const status = httpStatus(raw.statusCode) ?? httpStatus(fields(raw.response).status);
  const technicalMessage =
    text(body.technicalMessage) ??
    text(body.error) ??
    text(raw.message) ??
    text(value) ??
    UNKNOWN_TECHNICAL_MESSAGE;

  const message = apiErrorMessage(code);

  return Object.assign(new Error(message, { cause: value }), {
    name: 'ApiError',
    code,
    status,
    technicalMessage,
    statusCode: status,
    // The response body as received, plus the technical message of a failure that has no body.
    data: { ...body, technicalMessage },
  });
};
