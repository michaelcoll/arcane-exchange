import type { CardImportLineError } from '~/bindings/CardImportLineError';
import { apiErrorMessage } from './api-error';

const GENERIC_IMPORT_MESSAGE = "L'import a échoué.";

/** French label of each field a line error can name. */
const FIELD_LABELS: Record<string, string> = {
  set_code: 'set',
  collector_number: 'numéro de collection',
  rarity: 'rareté',
  language_code: 'langue',
  quantity: 'quantité',
  purchase_price: "prix d'achat",
  scryfall_id: 'identifiant Scryfall',
  added_at: "date d'ajout",
  proxy: 'proxy',
};

/**
 * Message of an import failure from its bare `code` (ADR 0018), read in the one table of
 * `api-error.ts`. Never the API's technical message: an unknown or missing code gets the import's
 * own generic one.
 */
export const importErrorMessage = (code: string | null | undefined): string =>
  apiErrorMessage(code, GENERIC_IMPORT_MESSAGE);

export const importLineErrorMessage = ({ line, field, value }: CardImportLineError): string =>
  `Ligne ${line} : ${FIELD_LABELS[field] ?? 'valeur'} invalide (${value})`;
