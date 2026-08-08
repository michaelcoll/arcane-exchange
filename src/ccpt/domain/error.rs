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
    PriceNotFound,
    CardNotFound,
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
            FunctionalError::PriceNotFound => "Price not found".to_string(),
            FunctionalError::CardNotFound => "Card not found".to_string(),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
