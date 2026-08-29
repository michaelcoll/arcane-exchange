use crate::domain::error::FunctionalError;

/// Number of items returned per page when the client does not specify `page_size`.
pub const DEFAULT_PAGE_SIZE: u32 = 20;

/// Highest `page_size` any endpoint accepts, regardless of its offset limit.
pub const MAX_PAGE_SIZE: u32 = 100;

/// A validated `page` / `page_size` pair.
///
/// The only way to obtain an instance is [`Pagination::try_new`], so a `Pagination` in hand is
/// always within bounds. `page` is 0-based and `page_size` is between 1 and [`MAX_PAGE_SIZE`].
/// Each caller supplies its own `max_offset`, since how deep an endpoint may be paginated is an
/// endpoint-specific business rule, not a global one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pagination {
    page: u32,
    page_size: u32,
    offset: u32,
}

impl Pagination {
    /// Validates `page` and `page_size` and computes the resulting offset against `max_offset`.
    ///
    /// # Errors
    ///
    /// Returns [`FunctionalError::InvalidPageSize`] when `page_size` is 0 or greater than
    /// [`MAX_PAGE_SIZE`], and [`FunctionalError::PaginationTooDeep`] when `page * page_size`
    /// exceeds `max_offset`.
    pub fn try_new(page: u32, page_size: u32, max_offset: u32) -> Result<Self, FunctionalError> {
        if page_size == 0 || page_size > MAX_PAGE_SIZE {
            return Err(FunctionalError::InvalidPageSize {
                requested: page_size,
                max: MAX_PAGE_SIZE,
            });
        }

        // Widen to u64 first: `page * page_size` as u32 could overflow, and the offset is only
        // ever compared against (or reported alongside) `max_offset`, never used as-is.
        let offset = u64::from(page) * u64::from(page_size);
        if offset > u64::from(max_offset) {
            return Err(FunctionalError::PaginationTooDeep {
                requested_offset: offset,
                max: max_offset,
            });
        }

        // offset <= max_offset, and max_offset is a small endpoint-defined constant that always
        // fits in a u32, so this narrowing cannot fail.
        let offset =
            u32::try_from(offset).expect("offset is bounded by max_offset, which fits in u32");

        Ok(Self {
            page,
            page_size,
            offset,
        })
    }

    pub fn page(&self) -> u32 {
        self.page
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    /// Number of items to skip, i.e. `page * page_size`.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Maximum number of items to return, i.e. `page_size`.
    pub fn limit(&self) -> u32 {
        self.page_size
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: DEFAULT_PAGE_SIZE,
            offset: 0,
        }
    }
}

/// A page of `T` together with the pagination that produced it and the total number of matching
/// items across all pages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub pagination: Pagination,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_with_zero_page_size_returns_invalid_page_size() {
        let result = Pagination::try_new(0, 0, 100);
        assert_eq!(
            result,
            Err(FunctionalError::InvalidPageSize {
                requested: 0,
                max: MAX_PAGE_SIZE
            })
        );
    }

    #[test]
    fn try_new_with_page_size_above_max_returns_invalid_page_size() {
        let result = Pagination::try_new(0, 101, 100);
        assert_eq!(
            result,
            Err(FunctionalError::InvalidPageSize {
                requested: 101,
                max: MAX_PAGE_SIZE
            })
        );
    }

    #[test]
    fn try_new_with_page_size_at_max_is_accepted() {
        let result = Pagination::try_new(0, 100, 100_000);
        assert!(result.is_ok());
    }

    #[test]
    fn try_new_with_offset_at_the_limit_is_accepted() {
        let result = Pagination::try_new(2, 20, 40);
        assert!(result.is_ok());
    }

    #[test]
    fn try_new_with_offset_above_the_limit_returns_pagination_too_deep() {
        let result = Pagination::try_new(3, 20, 40);
        assert_eq!(
            result,
            Err(FunctionalError::PaginationTooDeep {
                requested_offset: 60,
                max: 40
            })
        );
    }

    #[test]
    fn offset_is_page_times_page_size() {
        let pagination = Pagination::try_new(3, 20, 1000).unwrap();
        assert_eq!(pagination.offset(), 60);
    }

    #[test]
    fn limit_is_page_size() {
        let pagination = Pagination::try_new(3, 20, 1000).unwrap();
        assert_eq!(pagination.limit(), 20);
    }

    #[test]
    fn try_new_with_a_page_number_that_would_overflow_u32_reports_the_real_offset() {
        let result = Pagination::try_new(u32::MAX, 100, 1000);
        assert_eq!(
            result,
            Err(FunctionalError::PaginationTooDeep {
                requested_offset: u64::from(u32::MAX) * 100,
                max: 1000
            })
        );
    }

    #[test]
    fn default_pagination_is_page_zero_with_default_page_size() {
        let pagination = Pagination::default();
        assert_eq!(pagination.page(), 0);
        assert_eq!(pagination.page_size(), DEFAULT_PAGE_SIZE);
    }
}
