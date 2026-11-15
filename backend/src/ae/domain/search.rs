use crate::domain::card::CopyId;
use crate::domain::card_image::CardImage;
use crate::domain::collection::SortDirection;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::{PageRequest, Pagination};
use crate::domain::price::PriceGuide;
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::SetName;
use std::fmt;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum SearchSortField {
    Avg,
    #[default]
    Trend,
    SetCode,
    LanguageCode,
    AddedAt,
}

impl fmt::Display for SearchSortField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Trend => write!(f, "trend"),
            Self::SetCode => write!(f, "set_code"),
            Self::LanguageCode => write!(f, "language_code"),
            Self::AddedAt => write!(f, "added_at"),
        }
    }
}

/// A search across every player's tradable cards: text, rarity, sets, price range and an
/// optional exact-match filter on the owning player's username, sorted and paged.
///
/// `P` is the page: a raw [`PageRequest`] as it leaves the HTTP adapter, then a validated
/// [`Pagination`] once the use case has applied its own depth limit with
/// [`SearchQuery::paginate`] — only the latter reaches a repository.
#[derive(Clone, Debug, Default)]
pub struct SearchQuery<P = Pagination> {
    pub pagination: P,
    pub sort_by: SearchSortField,
    pub sort_dir: SortDirection,
    pub search_query: Option<String>,
    pub rarity: Vec<RarityCode>,
    pub sets: Vec<String>,
    pub price_min: Option<u32>,
    pub price_max: Option<u32>,
    pub player_username: Option<String>,
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
            sort_by: self.sort_by,
            sort_dir: self.sort_dir,
            search_query: self.search_query,
            rarity: self.rarity,
            sets: self.sets,
            price_min: self.price_min,
            price_max: self.price_max,
            player_username: self.player_username,
        })
    }
}

impl<P> SearchQuery<P> {
    /// Rejects combinations that don't make sense together.
    ///
    /// # Errors
    ///
    /// Returns [`FunctionalError::AddedAtSortRequiresPlayerUsername`] when `sort_by` is
    /// [`SearchSortField::AddedAt`] and no `player_username` is set: an unscoped search can
    /// group cards from several owners into a single row, so there is no single `added_at` value
    /// to sort by.
    pub fn validate(self) -> Result<Self, FunctionalError> {
        if self.sort_by == SearchSortField::AddedAt && self.player_username.is_none() {
            return Err(FunctionalError::AddedAtSortRequiresPlayerUsername);
        }

        Ok(self)
    }
}

/// What a search covers, without its filters: a text query, a player, or both. The search
/// facets are computed over this scope alone, so they stay stable while filters change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchScope {
    pub search_query: Option<String>,
    pub player_username: Option<String>,
}

/// A card a search found: what its owners offer for trade, reduced to what is public.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchCard {
    pub id: CopyId,
    pub set_name: SetName,
    pub name: String,
    pub rarity_code: RarityCode,
    pub scryfall_id: uuid::Uuid,
    /// `None` while the card's image is pending.
    pub image: Option<CardImage>,
    pub price_guide: Option<PriceGuide>,
    /// Number of distinct players offering this card for trade.
    pub owner_count: u64,
    /// `true` if this card is engaged in one of its owner's trades in `ONE_ACCEPTED` or
    /// `FULLY_ACCEPTED` status. Only meaningful when the search is scoped to a single player
    /// (`player_username` filter, hence `owner_count == 1`); `false` in the unscoped
    /// multi-owner aggregate, where no single owner can be attributed.
    pub reserved: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_sort_field_default_is_trend() {
        assert_eq!(SearchSortField::default(), SearchSortField::Trend);
    }

    #[test]
    fn search_sort_field_display() {
        assert_eq!(SearchSortField::Avg.to_string(), "avg");
        assert_eq!(SearchSortField::Trend.to_string(), "trend");
        assert_eq!(SearchSortField::SetCode.to_string(), "set_code");
        assert_eq!(SearchSortField::LanguageCode.to_string(), "language_code");
        assert_eq!(SearchSortField::AddedAt.to_string(), "added_at");
    }

    #[test]
    fn search_query_default_values() {
        let q: SearchQuery = SearchQuery::default();
        assert_eq!(q.pagination.page(), 0);
        assert_eq!(q.pagination.page_size(), 20);
        assert_eq!(q.sort_by, SearchSortField::Trend);
        assert_eq!(q.sort_dir, SortDirection::Desc);
        assert_eq!(q.search_query, None);
        assert_eq!(q.player_username, None);
    }

    #[test]
    fn search_query_paginate_validates_the_page_and_keeps_the_filters() {
        let query = SearchQuery {
            pagination: PageRequest {
                page: 2,
                page_size: 10,
            },
            sort_by: SearchSortField::SetCode,
            sets: vec!["FDN".to_string()],
            price_min: Some(100),
            player_username: Some("alice".to_string()),
            ..Default::default()
        };

        let paginated = query.paginate(20).unwrap();

        assert_eq!(
            paginated.pagination,
            Pagination::try_new(2, 10, 20).unwrap()
        );
        assert_eq!(paginated.sort_by, SearchSortField::SetCode);
        assert_eq!(paginated.sets, vec!["FDN".to_string()]);
        assert_eq!(paginated.price_min, Some(100));
        assert_eq!(paginated.player_username, Some("alice".to_string()));
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

    #[test]
    fn search_query_validate_rejects_added_at_sort_without_player_username() {
        let query = SearchQuery::<PageRequest> {
            sort_by: SearchSortField::AddedAt,
            ..Default::default()
        };

        assert!(matches!(
            query.validate(),
            Err(FunctionalError::AddedAtSortRequiresPlayerUsername)
        ));
    }

    #[test]
    fn search_query_validate_accepts_added_at_sort_with_player_username() {
        let query = SearchQuery::<PageRequest> {
            sort_by: SearchSortField::AddedAt,
            player_username: Some("alice".to_string()),
            ..Default::default()
        };

        assert_eq!(
            query.validate().unwrap().player_username,
            Some("alice".to_string())
        );
    }

    #[test]
    fn search_query_validate_accepts_non_added_at_sort_without_player_username() {
        assert!(SearchQuery::<PageRequest>::default().validate().is_ok());
    }
}
