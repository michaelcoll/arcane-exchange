use crate::domain::card::CardId;
use crate::domain::card_image::CardImage;
use crate::domain::collection::SortDirection;
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::{PageRequest, Pagination};
use crate::domain::price::PriceGuide;
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::SetName;

/// The Offre axis of a search: which impressions count by what players offer for trade.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Offer {
    /// Every impression of the Catalogue, offered or not.
    #[default]
    Any,
    /// Impressions offered by any player, the authenticated one included.
    Offered,
    /// Impressions offered by the player with this exact username (case-insensitive).
    OfferedBy(String),
}

/// The Possession axis of a search: which impressions count by what the authenticated player owns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Possession {
    /// Every impression of the Catalogue, owned or not.
    #[default]
    Any,
    /// Impressions the authenticated player owns, in any language or finish.
    Owned,
    /// Impressions the authenticated player owns in no language or finish.
    NotOwned,
}

/// A search of the Catalogue: text, Possession and Offre axes, rarity, sets and price range, sorted by
/// reference price (the normal trend price, or the foil one when there is no normal price) and
/// paged. With a text, the best name match comes first.
///
/// `P` is the page: a raw [`PageRequest`] as it leaves the HTTP adapter, then a validated
/// [`Pagination`] once the use case has applied its own depth limit with
/// [`SearchQuery::paginate`] — only the latter reaches a repository.
#[derive(Clone, Debug, Default)]
pub struct SearchQuery<P = Pagination> {
    pub pagination: P,
    pub sort_dir: SortDirection,
    pub search_query: Option<String>,
    pub possession: Possession,
    pub offer: Offer,
    pub rarity: Vec<RarityCode>,
    pub sets: Vec<String>,
    /// Exact collector number, combined with the other filters; with one set it names one
    /// impression.
    pub collector_number: Option<String>,
    pub price_min: Option<u32>,
    pub price_max: Option<u32>,
}

impl SearchQuery<PageRequest> {
    /// Validates the requested page against `max_offset`, keeping every other field.
    ///
    /// # Errors
    ///
    /// Same as [`PageRequest::paginate`].
    pub fn paginate(self, max_offset: u32) -> Result<SearchQuery, FunctionalError> {
        Ok(SearchQuery {
            pagination: self.pagination.paginate(max_offset)?,
            sort_dir: self.sort_dir,
            search_query: self.search_query,
            possession: self.possession,
            offer: self.offer,
            rarity: self.rarity,
            sets: self.sets,
            collector_number: self.collector_number,
            price_min: self.price_min,
            price_max: self.price_max,
        })
    }
}

/// What a search covers, without its filters: a text query and the two axes (Possession and
/// Offre). The search facets are computed over this scope alone, so they stay stable while
/// filters change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchScope {
    pub search_query: Option<String>,
    pub possession: Possession,
    pub offer: Offer,
}

/// The image of a search result: a downloaded image of one of its Cards, or the Scryfall one of
/// the Catalogue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchImage {
    /// The image of the Card `card_id` (French when one has an image, English otherwise).
    Local { card_id: CardId, image: CardImage },
    /// The Scryfall URL of the Catalogue, loaded by the client from Scryfall's CDN.
    Scryfall(String),
}

/// An impression a search found (set and number, every language and finish together), reduced
/// to what is public: no purchase price, no date of addition, nothing of another player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchCard {
    pub set_name: SetName,
    pub collector_number: String,
    pub rarity_code: RarityCode,
    pub name: String,
    pub french_name: Option<String>,
    pub scryfall_id: uuid::Uuid,
    /// `None` when the Catalogue has no image for the impression and none was downloaded.
    pub image: Option<SearchImage>,
    /// The normal price guide; `None` when Cardmarket has no normal price.
    pub price_guide: Option<PriceGuide>,
    /// The foil price guide; `None` when Cardmarket has no foil price.
    pub foil_price_guide: Option<PriceGuide>,
    /// Number of distinct players offering this impression for trade, the authenticated one
    /// included.
    pub owner_count: u64,
    /// Copies of the impression the authenticated player owns, every language and finish
    /// together; `0` when they own none. The only private data of a search.
    pub owned_quantity: u64,
    /// The languages offered for trade for the impression within the Offre scope of the search:
    /// by the searched player with [`Offer::OfferedBy`], by any player otherwise. Empty when
    /// nobody offers it.
    pub offered_languages: Vec<LanguageCode>,
    /// `true` if one of the Cards of the impression of the searched player is engaged in one of
    /// their trades in `ONE_ACCEPTED` or `FULLY_ACCEPTED` status. `None` unless the search is
    /// [`Offer::OfferedBy`]: no single player can be attributed otherwise.
    pub reserved: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_default_values() {
        let q: SearchQuery = SearchQuery::default();
        assert_eq!(q.pagination.page(), 0);
        assert_eq!(q.pagination.page_size(), 20);
        assert_eq!(q.sort_dir, SortDirection::Desc);
        assert_eq!(q.search_query, None);
        assert_eq!(q.possession, Possession::Any);
        assert_eq!(q.offer, Offer::Any);
    }

    #[test]
    fn search_query_paginate_validates_the_page_and_keeps_the_filters() {
        let query = SearchQuery {
            pagination: PageRequest {
                page: 2,
                page_size: 10,
            },
            sets: vec!["FDN".to_string()],
            price_min: Some(100),
            possession: Possession::NotOwned,
            offer: Offer::OfferedBy("alice".to_string()),
            ..Default::default()
        };

        let paginated = query.paginate(20).unwrap();

        assert_eq!(
            paginated.pagination,
            Pagination::try_new(2, 10, 20).unwrap()
        );
        assert_eq!(paginated.sets, vec!["FDN".to_string()]);
        assert_eq!(paginated.price_min, Some(100));
        assert_eq!(paginated.possession, Possession::NotOwned);
        assert_eq!(paginated.offer, Offer::OfferedBy("alice".to_string()));
    }

    #[test]
    fn search_query_paginate_rejects_a_page_beyond_max_offset() {
        let query = SearchQuery::<PageRequest> {
            pagination: PageRequest {
                page: 3,
                page_size: 10,
            },
            ..Default::default()
        };

        let result = query.paginate(20);

        assert!(matches!(
            result,
            Err(FunctionalError::PaginationTooDeep {
                requested_offset: 30,
                max: 20
            })
        ));
    }
}
