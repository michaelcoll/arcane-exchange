import { describe, it, expect } from 'vitest';
import { importErrorMessage, importLineErrorMessage } from './import-error';

describe('importErrorMessage', () => {
  it.each([
    ['empty_file', 'Le fichier est vide.'],
    ['malformed_csv', "Ce fichier n'est pas un CSV lisible."],
    [
      'binder_export',
      'Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox.',
    ],
    ['unrecognized_format', "Ce fichier n'est pas un export collection ManaBox reconnu."],
    ['no_valid_line', "Aucune ligne du fichier n'a pu être lue."],
    ['internal', 'Erreur technique, réessaie plus tard.'],
  ])('translates %s', (code, message) => {
    expect(importErrorMessage(code)).toBe(message);
  });

  it.each([['wrong_format'], [null], [undefined]])(
    'falls back to a generic message for %s',
    (code) => {
      expect(importErrorMessage(code)).toBe("L'import a échoué.");
    },
  );
});

describe('importLineErrorMessage', () => {
  it('names the field in French', () => {
    expect(importLineErrorMessage({ line: 2, field: 'language_code', value: 'xx' })).toBe(
      'Ligne 2 : langue invalide (xx)',
    );
  });

  it.each([
    ['set_code', 'set'],
    ['collector_number', 'numéro de collection'],
    ['rarity', 'rareté'],
    ['quantity', 'quantité'],
    ['purchase_price', "prix d'achat"],
    ['scryfall_id', 'identifiant Scryfall'],
    ['added_at', "date d'ajout"],
    ['proxy', 'proxy'],
  ])('labels %s as « %s »', (field, label) => {
    expect(importLineErrorMessage({ line: 3, field, value: 'v' })).toBe(
      `Ligne 3 : ${label} invalide (v)`,
    );
  });

  it('falls back to a generic label for an unknown field', () => {
    expect(importLineErrorMessage({ line: 4, field: 'foo', value: 'v' })).toBe(
      'Ligne 4 : valeur invalide (v)',
    );
  });
});
