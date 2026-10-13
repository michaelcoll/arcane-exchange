import { describe, it, expect } from 'vitest';
import { readApiError } from './api-error';

const REQUEST = '[POST] "/api/v1/trades/42/accept"';

/** What `$fetch` (ofetch `FetchError`) rejects with when the API answers an error. */
const fetchError = (status: number, data: unknown) =>
  Object.assign(new Error(`${REQUEST}: ${status} `), {
    name: 'FetchError',
    status,
    statusCode: status,
    statusText: '',
    statusMessage: '',
    response: { status, _data: data },
    data,
  });

/** What `$fetch` rejects with when the API does not answer: every response field is undefined. */
const networkError = () =>
  Object.assign(new Error(`${REQUEST}: <no response> fetch failed`), {
    name: 'FetchError',
    status: undefined,
    statusCode: undefined,
    response: undefined,
    data: undefined,
    cause: new TypeError('fetch failed'),
  });

/**
 * What `useAsyncData` exposes in `error` for a rejected handler, as observed on Nuxt 4.6:
 * `createError` builds a new error that keeps only `message`, `data` and `statusCode` (500 when
 * the rejected value has none). The SSR payload serializes those same three fields.
 */
const asyncDataError = (rejected: unknown) => {
  const { message, data, statusCode } = rejected as {
    message: string;
    data?: unknown;
    statusCode?: number;
  };
  return Object.assign(new Error(message), {
    statusCode: statusCode ?? 500,
    fatal: false,
    unhandled: false,
    statusMessage: undefined,
    data,
    cause: rejected,
  });
};

/** The same error once it went through the SSR payload: a plain object, without `cause`. */
const payloadError = (rejected: unknown) => {
  const { message, statusCode, data } = asyncDataError(rejected);
  return JSON.parse(JSON.stringify({ message, statusCode, data }));
};

const summary = (value: unknown) => {
  const { code, status, message, technicalMessage } = readApiError(value);
  return { code, status, message, technicalMessage };
};

describe('readApiError', () => {
  it('reads the code, the status and a French message from a raw $fetch error', () => {
    const error = fetchError(409, { error: 'This trade has no cards yet', code: 'trade_empty' });

    expect(summary(error)).toEqual({
      code: 'trade_empty',
      status: 409,
      message: "Ajoute au moins une carte avant d'accepter cet échange.",
      technicalMessage: 'This trade has no cards yet',
    });
  });

  it.each([
    ['self_trade', 'Tu ne peux pas ouvrir un échange avec toi-même.'],
    ['trade_not_modifiable', "Cet échange n'est plus modifiable."],
    ['trade_not_found', "Cet échange n'existe pas."],
    ['trade_access_denied', "Tu n'as pas accès à cet échange."],
    ['trade_not_acceptable', 'Cet échange ne peut plus être accepté.'],
    ['trade_already_accepted', 'Tu as déjà accepté cet échange.'],
    ['trade_already_finalized', 'Cet échange est terminé, il ne peut plus être abandonné.'],
    [
      'trade_not_fully_accepted',
      "Les deux joueurs doivent accepter l'échange avant de le confirmer.",
    ],
    ['trade_already_confirmed', 'Tu as déjà confirmé cet échange.'],
    ['trade_not_completed', "Tu pourras noter cet échange une fois qu'il sera terminé."],
    ['trade_already_rated', 'Tu as déjà noté cet échange.'],
    [
      'trade_concurrently_modified',
      "L'échange vient d'être modifié. Recharge la page avant de recommencer.",
    ],
    ['trade_empty', "Ajoute au moins une carte avant d'accepter cet échange."],
    ['trade_card_not_found', "Cette carte ne fait plus partie de l'échange."],
    ['card_already_reserved', 'Cette carte est déjà réservée dans un autre échange.'],
    ['binder_not_found', "Ce binder n'existe plus dans ta collection."],
    ['user_not_found', 'Ce joueur est introuvable.'],
    ['card_not_found', "Cette carte est introuvable ou n'est plus disponible."],
    ['set_not_found', 'Ce set est introuvable.'],
    ['unauthorized', 'Ta session a expiré, reconnecte-toi.'],
    ['internal', 'Erreur technique, réessaie plus tard.'],
  ])('translates %s', (code, message) => {
    expect(readApiError(fetchError(400, { error: 'Technical', code })).message).toBe(message);
  });

  it.each([
    ['an unknown code', fetchError(400, { error: 'Invalid page size', code: 'invalid_page_size' })],
    ['a code named like an object member', fetchError(400, { error: 'Odd', code: 'toString' })],
    ['a missing code', fetchError(502, { error: 'Bad gateway' })],
    ['a body that is not JSON', fetchError(502, '<html>Bad gateway</html>')],
    ['a network failure', networkError()],
    ['a plain Error', new Error('Cannot read properties of undefined')],
    ['a string', 'boom'],
    ['null', null],
    ['undefined', undefined],
  ])('falls back to the generic message for %s', (_, value) => {
    expect(readApiError(value).message).toBe('Une erreur est survenue.');
  });

  it('keeps an unknown code readable although it has no message', () => {
    const error = fetchError(400, { error: 'Invalid page size', code: 'invalid_page_size' });

    expect(readApiError(error).code).toBe('invalid_page_size');
  });

  it('reads the status from `statusCode`', () => {
    expect(readApiError({ statusCode: 404 }).status).toBe(404);
  });

  it('reads the status from `response.status`', () => {
    expect(readApiError({ response: { status: 403 } }).status).toBe(403);
  });

  it('gives a network failure neither code nor status', () => {
    expect(summary(networkError())).toEqual({
      code: undefined,
      status: undefined,
      message: 'Une erreur est survenue.',
      technicalMessage: `${REQUEST}: <no response> fetch failed`,
    });
  });

  it('gives a value that is not an error neither code nor status', () => {
    expect(summary(null)).toEqual({
      code: undefined,
      status: undefined,
      message: 'Une erreur est survenue.',
      technicalMessage: 'Unknown error',
    });
  });

  describe('gives the same result on every path an error takes', () => {
    const raw = fetchError(409, {
      error: 'This card is already reserved by another trade',
      code: 'card_already_reserved',
    });
    const expected = {
      code: 'card_already_reserved',
      status: 409,
      message: 'Cette carte est déjà réservée dans un autre échange.',
      technicalMessage: 'This card is already reserved by another trade',
    };

    it.each([
      ['a raw $fetch error', raw],
      ['an error already read', readApiError(raw)],
      ['an error read twice', readApiError(readApiError(raw))],
      ['a raw $fetch error exposed by useAsyncData', asyncDataError(raw)],
      ['an error already read, exposed by useAsyncData', asyncDataError(readApiError(raw))],
      ['an error already read, back from the SSR payload', payloadError(readApiError(raw))],
    ])('for %s', (_, value) => {
      expect(summary(value)).toEqual(expected);
    });
  });

  it('hands the response body over unchanged, for the callers that still read it', () => {
    const body = { error: 'Trade not found', code: 'trade_not_found' };

    expect(readApiError(fetchError(404, body))).toMatchObject({ statusCode: 404, data: body });
    expect(readApiError(networkError())).not.toHaveProperty('data.error');
  });

  it('keeps the technical message of an error without code across useAsyncData', () => {
    const exposed = asyncDataError(readApiError(networkError()));

    expect(readApiError(exposed)).toMatchObject({
      code: undefined,
      message: 'Une erreur est survenue.',
      technicalMessage: `${REQUEST}: <no response> fetch failed`,
    });
  });
});
