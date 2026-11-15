import type { CollectionCard } from '~/bindings/CollectionCard';
import type { SearchCard as SearchCardBinding } from '~/bindings/SearchCard';
import type { DeepReadonly } from 'vue';

/** A result as the list hands it out: read-only, like what Nuxt keeps in its state. */
type SearchCard = DeepReadonly<SearchCardBinding>;

/** The name a result shows: French when known, English otherwise. */
export const searchCardName = (card: SearchCard): string => card.french_name ?? card.name;

/** An impression with no normal price but a foil one: its reference price is the foil trend. */
export const isFoilOnly = (card: SearchCard): boolean =>
  card.price_guide?.trend == null && card.foil_price_guide?.trend != null;

/** The reference price of a result (cents): the normal trend, or the foil one without it. */
export const searchCardPrice = (card: SearchCard): number | undefined =>
  card.price_guide?.trend ?? card.foil_price_guide?.trend ?? undefined;

/**
 * The result as the card detail reads it. The detail looks at one Card, with a language and a
 * finish, which an impression has none of: it opens on the French one when it is offered, the
 * English one otherwise, in foil when the impression only has a foil price.
 */
export const searchCardToDetail = (card: SearchCard): CollectionCard => {
  const foil = isFoilOnly(card);
  return {
    set_code: card.set_code,
    collector_number: card.collector_number,
    language_code: card.offered_languages.includes('FR') ? 'FR' : 'EN',
    foil,
    name: searchCardName(card),
    rarity_code: card.rarity_code,
    scryfall_id: card.scryfall_id,
    image_url: card.image_url,
    image_back_url: card.image_back_url,
    collection_entry: null,
    reserved: card.reserved ?? false,
    price_guide: (foil ? card.foil_price_guide : card.price_guide) ?? null,
  };
};
