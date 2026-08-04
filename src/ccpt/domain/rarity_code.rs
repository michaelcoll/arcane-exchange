use crate::domain::error::FunctionalError;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RarityCode {
    C,
    U,
    R,
    M,
    S,
}

impl RarityCode {
    pub fn try_new<S: AsRef<str>>(s: S) -> Result<Self, FunctionalError> {
        let s_ref = s.as_ref();
        match s_ref.to_lowercase().as_str() {
            "common" | "c" => Ok(RarityCode::C),
            "uncommon" | "u" => Ok(RarityCode::U),
            "rare" | "r" => Ok(RarityCode::R),
            "mythic" | "m" => Ok(RarityCode::M),
            "special" | "s" => Ok(RarityCode::S),
            _ => Err(FunctionalError::InvalidRarityCode(s_ref.to_string())),
        }
    }
}

impl Display for RarityCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RarityCode::C => write!(f, "C"),
            RarityCode::U => write!(f, "U"),
            RarityCode::R => write!(f, "R"),
            RarityCode::M => write!(f, "M"),
            RarityCode::S => write!(f, "S"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_returns_common_for_common_string() {
        assert_eq!(RarityCode::try_new("common"), Ok(RarityCode::C));
    }

    #[test]
    fn try_new_returns_uncommon_for_uncommon_string() {
        assert_eq!(RarityCode::try_new("uncommon"), Ok(RarityCode::U));
    }

    #[test]
    fn try_new_returns_rare_for_rare_string() {
        assert_eq!(RarityCode::try_new("rare"), Ok(RarityCode::R));
    }

    #[test]
    fn try_new_returns_mythic_for_mythic_string() {
        assert_eq!(RarityCode::try_new("mythic"), Ok(RarityCode::M));
    }

    #[test]
    fn try_new_returns_special_for_special_string() {
        assert_eq!(RarityCode::try_new("special"), Ok(RarityCode::S));
    }

    #[test]
    fn try_new_is_case_insensitive_for_common() {
        assert_eq!(RarityCode::try_new("Common"), Ok(RarityCode::C));
        assert_eq!(RarityCode::try_new("COMMON"), Ok(RarityCode::C));
    }

    #[test]
    fn try_new_is_case_insensitive_for_uncommon() {
        assert_eq!(RarityCode::try_new("Uncommon"), Ok(RarityCode::U));
        assert_eq!(RarityCode::try_new("UNCOMMON"), Ok(RarityCode::U));
    }

    #[test]
    fn try_new_is_case_insensitive_for_rare() {
        assert_eq!(RarityCode::try_new("Rare"), Ok(RarityCode::R));
        assert_eq!(RarityCode::try_new("RARE"), Ok(RarityCode::R));
    }

    #[test]
    fn try_new_is_case_insensitive_for_mythic() {
        assert_eq!(RarityCode::try_new("Mythic"), Ok(RarityCode::M));
        assert_eq!(RarityCode::try_new("MYTHIC"), Ok(RarityCode::M));
    }

    #[test]
    fn try_new_is_case_insensitive_for_special() {
        assert_eq!(RarityCode::try_new("Special"), Ok(RarityCode::S));
        assert_eq!(RarityCode::try_new("SPECIAL"), Ok(RarityCode::S));
    }

    #[test]
    fn try_new_returns_error_for_unknown_rarity() {
        let result = RarityCode::try_new("joke");
        assert!(result.is_err());
    }

    #[test]
    fn try_new_returns_error_for_empty_string() {
        let result = RarityCode::try_new("");
        assert!(result.is_err());
    }

    #[test]
    fn try_new_returns_invalid_rarity_code_error_variant() {
        let result = RarityCode::try_new("joke");
        match result {
            Err(FunctionalError::InvalidRarityCode(msg)) => assert_eq!(msg, "joke"),
            _ => panic!("Expected InvalidRarityCode variant"),
        }
    }

    #[test]
    fn display_formats_common_as_c() {
        assert_eq!(RarityCode::C.to_string(), "C");
    }

    #[test]
    fn display_formats_uncommon_as_u() {
        assert_eq!(RarityCode::U.to_string(), "U");
    }

    #[test]
    fn display_formats_rare_as_r() {
        assert_eq!(RarityCode::R.to_string(), "R");
    }

    #[test]
    fn display_formats_mythic_as_m() {
        assert_eq!(RarityCode::M.to_string(), "M");
    }

    #[test]
    fn display_formats_special_as_s() {
        assert_eq!(RarityCode::S.to_string(), "S");
    }
}
