import type { CardImportLineError } from '~/bindings/CardImportLineError';

/** User-facing message for each import error `code` the API returns (ADR 0018). */
const MESSAGES: Record<string, string> = {
  empty_file: 'Le fichier est vide.',
  malformed_csv: "Ce fichier n'est pas un CSV lisible.",
  binder_export:
    'Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox.',
  unrecognized_format: "Ce fichier n'est pas un export collection ManaBox reconnu.",
  no_valid_line: "Aucune ligne du fichier n'a pu être lue.",
  internal: 'Erreur technique, réessaie plus tard.',
};

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

/** Never the API's technical message: an unknown or missing code gets a generic one. */
export const importErrorMessage = (code: string | null | undefined): string =>
  (code && MESSAGES[code]) || "L'import a échoué.";

export const importLineErrorMessage = ({ line, field, value }: CardImportLineError): string =>
  `Ligne ${line} : ${FIELD_LABELS[field] ?? 'valeur'} invalide (${value})`;
