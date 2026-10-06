use chrono::NaiveDate;
use std::ops::AddAssign;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Price {
    pub value: Option<u32>,
}

impl Price {
    pub fn empty() -> Self {
        Self { value: None }
    }
}

impl AddAssign for Price {
    fn add_assign(&mut self, other: Self) {
        self.value = match (self.value, other.value) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriceGuide {
    /// Low price in cents
    pub low: Price,
    /// Average price in cents
    pub avg: Price,
    /// Trend price in cents
    pub trend: Price,
}

/// Represents one day's aggregated price for a user's collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceHistoryEntry {
    pub date: NaiveDate,
    pub price_guide: PriceGuide,
}

pub struct FullPriceGuide {
    pub id_product: u32,
    pub normal: PriceGuide,
    pub foil: PriceGuide,
}

impl PriceGuide {
    pub fn new(low: impl Into<Price>, trend: impl Into<Price>, avg: impl Into<Price>) -> Self {
        Self {
            low: low.into(),
            trend: trend.into(),
            avg: avg.into(),
        }
    }
}

impl AddAssign for PriceGuide {
    fn add_assign(&mut self, other: Self) {
        self.low += other.low;
        self.trend += other.trend;
        self.avg += other.avg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl From<u32> for Price {
        fn from(value: u32) -> Self {
            Price { value: Some(value) }
        }
    }

    fn create_price(
        low: impl Into<Price>,
        trend: impl Into<Price>,
        avg: impl Into<Price>,
    ) -> PriceGuide {
        PriceGuide {
            low: low.into(),
            trend: trend.into(),
            avg: avg.into(),
        }
    }

    #[test]
    fn add_assign_combines_prices_correctly() {
        let mut price1 = create_price(100, 200, 200);
        let price2 = create_price(50, 100, 100);

        price1 += price2;

        assert_eq!(price1, create_price(150, 300, 300));
    }

    #[test]
    fn add_assign_with_zero_price_does_not_change_values() {
        let mut price1 = PriceGuide::new(Price::empty(), Price::empty(), Price::empty());
        let price2 = create_price(100, 200, 200);

        price1 += price2;

        assert_eq!(price1, create_price(100, 200, 200));
    }
}
