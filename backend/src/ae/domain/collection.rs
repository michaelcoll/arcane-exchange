use crate::domain::error::FunctionalError;
use crate::domain::pagination::{PageRequest, Pagination};
use crate::domain::rarity_code::RarityCode;
use std::fmt;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum CollectionSortField {
    Avg,
    #[default]
    Trend,
    SetCode,
    LanguageCode,
    AddedAt,
}

impl fmt::Display for CollectionSortField {
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

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

impl fmt::Display for SortDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asc => write!(f, "ASC"),
            Self::Desc => write!(f, "DESC"),
        }
    }
}

/// Filters, sort and page of a card listing.
///
/// `P` is the page: a raw [`PageRequest`] as it leaves the HTTP adapter, then a validated
/// [`Pagination`] once the use case has applied its own depth limit with
/// [`CollectionQuery::paginate`] — only the latter reaches a repository.
#[derive(Clone, Debug, Default)]
pub struct CollectionQuery<P = Pagination> {
    pub pagination: P,
    pub sort_by: CollectionSortField,
    pub sort_dir: SortDirection,
    pub search_query: Option<String>,
    pub rarity: Vec<RarityCode>,
    pub sets: Vec<String>,
    pub price_min: Option<u32>,
    pub price_max: Option<u32>,
}

impl CollectionQuery<PageRequest> {
    /// Validates the requested page against `max_offset`, keeping every other field.
    ///
    /// # Errors
    ///
    /// Same as [`PageRequest::paginate`].
    pub fn paginate(self, max_offset: u32) -> Result<CollectionQuery, FunctionalError> {
        Ok(CollectionQuery {
            pagination: self.pagination.paginate(max_offset)?,
            sort_by: self.sort_by,
            sort_dir: self.sort_dir,
            search_query: self.search_query,
            rarity: self.rarity,
            sets: self.sets,
            price_min: self.price_min,
            price_max: self.price_max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_sort_field_default_is_avg() {
        assert_eq!(CollectionSortField::default(), CollectionSortField::Trend);
    }

    #[test]
    fn collection_sort_field_display() {
        assert_eq!(CollectionSortField::Avg.to_string(), "avg");
        assert_eq!(CollectionSortField::SetCode.to_string(), "set_code");
        assert_eq!(
            CollectionSortField::LanguageCode.to_string(),
            "language_code"
        );
        assert_eq!(CollectionSortField::AddedAt.to_string(), "added_at");
    }

    #[test]
    fn sort_direction_default_is_desc() {
        assert_eq!(SortDirection::default(), SortDirection::Desc);
    }

    #[test]
    fn sort_direction_display() {
        assert_eq!(SortDirection::Asc.to_string(), "ASC");
        assert_eq!(SortDirection::Desc.to_string(), "DESC");
    }

    #[test]
    fn collection_query_default_values() {
        let q: CollectionQuery = CollectionQuery::default();
        assert_eq!(q.pagination.page(), 0);
        assert_eq!(q.pagination.page_size(), 20);
        assert_eq!(q.sort_by, CollectionSortField::Trend);
        assert_eq!(q.sort_dir, SortDirection::Desc);
        assert_eq!(q.search_query, None);
    }

    #[test]
    fn collection_query_paginate_validates_the_page_and_keeps_the_filters() {
        let query = CollectionQuery {
            pagination: PageRequest {
                page: 2,
                page_size: 10,
            },
            sort_by: CollectionSortField::SetCode,
            sets: vec!["FDN".to_string()],
            price_min: Some(100),
            ..Default::default()
        };

        let paginated = query.paginate(20).unwrap();

        assert_eq!(
            paginated.pagination,
            Pagination::try_new(2, 10, 20).unwrap()
        );
        assert_eq!(paginated.sort_by, CollectionSortField::SetCode);
        assert_eq!(paginated.sets, vec!["FDN".to_string()]);
        assert_eq!(paginated.price_min, Some(100));
    }

    #[test]
    fn collection_query_paginate_rejects_a_page_beyond_max_offset() {
        let query = CollectionQuery {
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
