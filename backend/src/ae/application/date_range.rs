use crate::application::error::AppError;
use crate::domain::error::FunctionalError;
use chrono::{Days, NaiveDate, Utc};

/// Resolves an optional `(start_date, end_date)` pair into concrete dates.
///
/// `end_date` defaults to today, `start_date` defaults to `end_date` minus 30 days. Returns
/// `AppError::WrongFormat` if the resolved `start_date` is after the resolved `end_date`.
pub(crate) fn resolve_date_range(
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> Result<(NaiveDate, NaiveDate), AppError> {
    let end_date = end_date.unwrap_or_else(|| Utc::now().date_naive());
    let start_date = start_date.unwrap_or_else(|| end_date - Days::new(30));

    if start_date > end_date {
        return Err(FunctionalError::WrongFormat(
            "start_date must be before or equal to end_date".to_string(),
        )
        .into());
    }

    Ok((start_date, end_date))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn defaults_to_last_30_days_when_no_dates_provided() {
        let today = Utc::now().date_naive();

        let (start, end) = resolve_date_range(None, None).unwrap();

        assert_eq!(end, today);
        assert_eq!(start, today - Days::new(30));
    }

    #[test]
    fn defaults_start_date_to_end_date_minus_30_days_when_only_end_date_provided() {
        let end_input = date(2025, 6, 30);

        let (start, end) = resolve_date_range(None, Some(end_input)).unwrap();

        assert_eq!(end, end_input);
        assert_eq!(start, end_input - Days::new(30));
    }

    #[test]
    fn defaults_end_date_to_today_when_only_start_date_provided() {
        let start_input = date(2025, 6, 1);
        let today = Utc::now().date_naive();

        let (start, end) = resolve_date_range(Some(start_input), None).unwrap();

        assert_eq!(start, start_input);
        assert_eq!(end, today);
    }

    #[test]
    fn returns_both_dates_unchanged_when_both_provided() {
        let (start, end) =
            resolve_date_range(Some(date(2025, 1, 1)), Some(date(2025, 1, 31))).unwrap();

        assert_eq!(start, date(2025, 1, 1));
        assert_eq!(end, date(2025, 1, 31));
    }

    #[test]
    fn accepts_same_start_and_end_date() {
        let d = date(2025, 6, 1);

        let result = resolve_date_range(Some(d), Some(d));

        assert!(result.is_ok());
    }

    #[test]
    fn returns_error_when_start_after_end() {
        let result = resolve_date_range(Some(date(2025, 2, 1)), Some(date(2025, 1, 1)));

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Functional(FunctionalError::WrongFormat(msg)) => {
                assert_eq!(msg, "start_date must be before or equal to end_date");
            }
            other => panic!("Expected WrongFormat, got {:?}", other),
        }
    }
}
