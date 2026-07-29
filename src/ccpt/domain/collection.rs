use crate::domain::card::Card;
use crate::domain::rarity_code::RarityCode;
use std::fmt;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum CollectionSortField {
    Avg,
    #[default]
    Trend,
    SetCode,
    LanguageCode,
}

impl fmt::Display for CollectionSortField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Trend => write!(f, "trend"),
            Self::SetCode => write!(f, "set_code"),
            Self::LanguageCode => write!(f, "language_code"),
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

#[derive(Clone, Debug)]
pub struct CollectionQuery {
    pub page: u32,
    pub page_size: u32,
    pub sort_by: CollectionSortField,
    pub sort_dir: SortDirection,
    pub search_query: Option<String>,
    pub rarity: Vec<RarityCode>,
    pub sets: Vec<String>,
    pub price_min: Option<u32>,
    pub price_max: Option<u32>,
}

impl Default for CollectionQuery {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 20,
            sort_by: CollectionSortField::default(),
            sort_dir: SortDirection::default(),
            search_query: None,
            rarity: Vec::new(),
            sets: Vec::new(),
            price_min: None,
            price_max: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaginatedCollection {
    pub items: Vec<Card>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
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
        let q = CollectionQuery::default();
        assert_eq!(q.page, 0);
        assert_eq!(q.page_size, 20);
        assert_eq!(q.sort_by, CollectionSortField::Trend);
        assert_eq!(q.sort_dir, SortDirection::Desc);
        assert_eq!(q.search_query, None);
    }
}
