#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionalError {
    ParseError {
        line: usize,
        field: &'static str,
        value: String,
    },
    InvalidLanguageCode(String),
    InvalidSetCode(String),
    InvalidRarityCode(String),
    InvalidCollectorNumber(String),
    WrongFormat(String),
    /// The imported file has no data line (or no header).
    EmptyFile,
    /// The imported file is not readable as CSV; carries the reader's diagnostic.
    MalformedCsv(String),
    /// A ManaBox binder export was imported where a collection export is expected.
    BinderExport,
    /// The header is not the one of a ManaBox collection export.
    UnrecognizedFormat {
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    /// Not a single line of the imported file could be read.
    NoValidLine,
    InvalidPageSize {
        requested: u32,
        max: u32,
    },
    PaginationTooDeep {
        requested_offset: u64,
        max: u32,
    },
    AddedAtSortRequiresPlayerUsername,
    PriceNotFound,
    CardNotFound,
    SetNotFound,
    SelfTrade,
    TradeNotModifiable,
    TradeNotFound,
    TradeAccessDenied,
    TradeNotAcceptable,
    TradeAlreadyAccepted,
    TradeAlreadyFinalized,
    TradeNotFullyAccepted,
    TradeAlreadyConfirmed,
    TradeNotCompleted,
    TradeAlreadyRated,
    TradeConcurrentlyModified,
    UserNotFound,
    TradeEmpty,
    TradeCardNotFound,
    CardAlreadyReserved,
    BinderNotFound,
    InvalidCardImportStatus(String),
    ImportAlreadyRunning,
    ImportNotFound,
}

impl FunctionalError {
    /// Stable identifier of the error, part of the API contract: clients translate it into a
    /// message, so renaming one breaks the clients already deployed (ADR 0018).
    pub fn code(&self) -> &'static str {
        match self {
            FunctionalError::ParseError { .. } => "parse_error",
            FunctionalError::InvalidLanguageCode(_) => "invalid_language_code",
            FunctionalError::InvalidSetCode(_) => "invalid_set_code",
            FunctionalError::InvalidRarityCode(_) => "invalid_rarity_code",
            FunctionalError::InvalidCollectorNumber(_) => "invalid_collector_number",
            FunctionalError::WrongFormat(_) => "wrong_format",
            FunctionalError::EmptyFile => "empty_file",
            FunctionalError::MalformedCsv(_) => "malformed_csv",
            FunctionalError::BinderExport => "binder_export",
            FunctionalError::UnrecognizedFormat { .. } => "unrecognized_format",
            FunctionalError::NoValidLine => "no_valid_line",
            FunctionalError::InvalidPageSize { .. } => "invalid_page_size",
            FunctionalError::PaginationTooDeep { .. } => "pagination_too_deep",
            FunctionalError::AddedAtSortRequiresPlayerUsername => {
                "added_at_sort_requires_player_username"
            }
            FunctionalError::PriceNotFound => "price_not_found",
            FunctionalError::CardNotFound => "card_not_found",
            FunctionalError::SetNotFound => "set_not_found",
            FunctionalError::SelfTrade => "self_trade",
            FunctionalError::TradeNotModifiable => "trade_not_modifiable",
            FunctionalError::TradeNotFound => "trade_not_found",
            FunctionalError::TradeAccessDenied => "trade_access_denied",
            FunctionalError::TradeNotAcceptable => "trade_not_acceptable",
            FunctionalError::TradeAlreadyAccepted => "trade_already_accepted",
            FunctionalError::TradeAlreadyFinalized => "trade_already_finalized",
            FunctionalError::TradeNotFullyAccepted => "trade_not_fully_accepted",
            FunctionalError::TradeAlreadyConfirmed => "trade_already_confirmed",
            FunctionalError::TradeNotCompleted => "trade_not_completed",
            FunctionalError::TradeAlreadyRated => "trade_already_rated",
            FunctionalError::TradeConcurrentlyModified => "trade_concurrently_modified",
            FunctionalError::UserNotFound => "user_not_found",
            FunctionalError::TradeEmpty => "trade_empty",
            FunctionalError::TradeCardNotFound => "trade_card_not_found",
            FunctionalError::CardAlreadyReserved => "card_already_reserved",
            FunctionalError::BinderNotFound => "binder_not_found",
            FunctionalError::InvalidCardImportStatus(_) => "invalid_card_import_status",
            FunctionalError::ImportAlreadyRunning => "import_already_running",
            FunctionalError::ImportNotFound => "import_not_found",
        }
    }
}

impl From<FunctionalError> for String {
    fn from(val: FunctionalError) -> String {
        match val {
            FunctionalError::ParseError { line, field, value } => format!(
                "Line {}: invalid {} '{}' (must be a valid value)",
                line, field, value
            ),
            FunctionalError::InvalidLanguageCode(msg) => format!("Invalid language code '{}'", msg),
            FunctionalError::InvalidSetCode(msg) => format!("Invalid set code '{}'", msg),
            FunctionalError::InvalidRarityCode(msg) => format!("Invalid rarity code '{}'", msg),
            FunctionalError::InvalidCollectorNumber(msg) => msg,
            FunctionalError::WrongFormat(msg) => msg,
            FunctionalError::EmptyFile => "missing headers or empty file".to_string(),
            FunctionalError::MalformedCsv(msg) => format!("malformed CSV: {msg}"),
            FunctionalError::BinderExport => {
                "expecting a collection export, got a binder export".to_string()
            }
            FunctionalError::UnrecognizedFormat {
                missing,
                unexpected,
            } => format!(
                "unrecognized collection export: missing columns [{}], unexpected columns [{}]",
                missing.join(", "),
                unexpected.join(", ")
            ),
            FunctionalError::NoValidLine => "no valid line in file".to_string(),
            FunctionalError::InvalidPageSize { requested, max } => {
                format!("Invalid page_size '{requested}' (must be between 1 and {max})")
            }
            FunctionalError::PaginationTooDeep {
                requested_offset,
                max,
            } => format!(
                "Pagination too deep: requested offset '{requested_offset}' exceeds the maximum of {max} for this endpoint"
            ),
            FunctionalError::AddedAtSortRequiresPlayerUsername => {
                "Sorting by 'added_at' on /search/card requires player_username to be set"
                    .to_string()
            }
            FunctionalError::PriceNotFound => "Price not found".to_string(),
            FunctionalError::CardNotFound => "Card not found".to_string(),
            FunctionalError::SetNotFound => "Set not found".to_string(),
            FunctionalError::SelfTrade => "Cannot request your own card".to_string(),
            FunctionalError::TradeNotModifiable => {
                "This trade has already been fully accepted and can no longer be modified"
                    .to_string()
            }
            FunctionalError::TradeNotFound => "Trade not found".to_string(),
            FunctionalError::TradeAccessDenied => "You are not a party to this trade".to_string(),
            FunctionalError::TradeNotAcceptable => {
                "This trade cannot be accepted in its current status".to_string()
            }
            FunctionalError::TradeAlreadyAccepted => {
                "You have already accepted this trade".to_string()
            }
            FunctionalError::TradeAlreadyFinalized => {
                "This trade is already finalized and cannot be abandoned".to_string()
            }
            FunctionalError::TradeNotFullyAccepted => {
                "This trade must be fully accepted before it can be confirmed".to_string()
            }
            FunctionalError::TradeAlreadyConfirmed => {
                "You have already confirmed this trade".to_string()
            }
            FunctionalError::TradeNotCompleted => {
                "This trade must be completed before it can be rated".to_string()
            }
            FunctionalError::TradeAlreadyRated => "You have already rated this trade".to_string(),
            FunctionalError::TradeConcurrentlyModified => {
                "This trade is being modified by another request, please retry".to_string()
            }
            FunctionalError::UserNotFound => "User not found".to_string(),
            FunctionalError::TradeEmpty => {
                "This trade has no cards yet and cannot be accepted".to_string()
            }
            FunctionalError::TradeCardNotFound => "This card is not part of the trade".to_string(),
            FunctionalError::CardAlreadyReserved => {
                "This card is already reserved by another trade".to_string()
            }
            FunctionalError::BinderNotFound => "Binder not found in your collection".to_string(),
            FunctionalError::InvalidCardImportStatus(msg) => {
                format!("Invalid card import status '{}'", msg)
            }
            FunctionalError::ImportAlreadyRunning => {
                "An import is already in progress for this user".to_string()
            }
            FunctionalError::ImportNotFound => "Import not found".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_format_errors_carry_their_stable_code() {
        let cases = [
            (FunctionalError::EmptyFile, "empty_file"),
            (
                FunctionalError::MalformedCsv("bad quote".to_string()),
                "malformed_csv",
            ),
            (FunctionalError::BinderExport, "binder_export"),
            (
                FunctionalError::UnrecognizedFormat {
                    missing: vec![],
                    unexpected: vec!["Tags".to_string()],
                },
                "unrecognized_format",
            ),
            (FunctionalError::NoValidLine, "no_valid_line"),
        ];

        for (error, code) in cases {
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn code_is_the_snake_case_variant_name() {
        assert_eq!(
            FunctionalError::WrongFormat(String::new()).code(),
            "wrong_format"
        );
        assert_eq!(
            FunctionalError::InvalidPageSize {
                requested: 0,
                max: 1
            }
            .code(),
            "invalid_page_size"
        );
        assert_eq!(
            FunctionalError::ImportAlreadyRunning.code(),
            "import_already_running"
        );
    }

    #[test]
    fn string_from_unrecognized_format_lists_the_columns() {
        let msg: String = FunctionalError::UnrecognizedFormat {
            missing: vec!["Signed".to_string(), "Proxy".to_string()],
            unexpected: vec![],
        }
        .into();
        assert_eq!(
            msg,
            "unrecognized collection export: missing columns [Signed, Proxy], unexpected columns []"
        );
    }

    #[test]
    fn string_from_invalid_language_code_includes_the_value() {
        let msg: String = FunctionalError::InvalidLanguageCode("XX".to_string()).into();
        assert_eq!(msg, "Invalid language code 'XX'");
    }

    #[test]
    fn string_from_invalid_set_code_includes_the_value() {
        let msg: String = FunctionalError::InvalidSetCode("AB".to_string()).into();
        assert_eq!(msg, "Invalid set code 'AB'");
    }

    #[test]
    fn string_from_invalid_rarity_code_includes_the_value() {
        let msg: String = FunctionalError::InvalidRarityCode("joke".to_string()).into();
        assert_eq!(msg, "Invalid rarity code 'joke'");
    }

    #[test]
    fn string_from_invalid_collector_number_is_the_message_as_is() {
        let msg: String = FunctionalError::InvalidCollectorNumber(
            "collector number must be 10 characters or less (got X)".to_string(),
        )
        .into();
        assert_eq!(
            msg,
            "collector number must be 10 characters or less (got X)"
        );
    }

    #[test]
    fn string_from_invalid_page_size_mentions_requested_value_and_max() {
        let msg: String = FunctionalError::InvalidPageSize {
            requested: 500,
            max: 100,
        }
        .into();
        assert_eq!(msg, "Invalid page_size '500' (must be between 1 and 100)");
    }

    #[test]
    fn string_from_pagination_too_deep_mentions_requested_offset_and_max() {
        let msg: String = FunctionalError::PaginationTooDeep {
            requested_offset: 20000,
            max: 60,
        }
        .into();
        assert_eq!(
            msg,
            "Pagination too deep: requested offset '20000' exceeds the maximum of 60 for this endpoint"
        );
    }

    #[test]
    fn string_from_added_at_sort_requires_player_username_is_descriptive() {
        let msg: String = FunctionalError::AddedAtSortRequiresPlayerUsername.into();
        assert_eq!(
            msg,
            "Sorting by 'added_at' on /search/card requires player_username to be set"
        );
    }
}
