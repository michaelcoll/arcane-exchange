use crate::domain::error::FunctionalError;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LanguageCode {
    DE,
    EN,
    FR,
    IT,
    JA,
    SP,
}

impl LanguageCode {
    pub fn try_new<S: AsRef<str>>(s: S) -> Result<Self, FunctionalError> {
        let s_ref = s.as_ref();
        match s_ref.to_uppercase().as_str() {
            "DE" => Ok(LanguageCode::DE),
            "EN" => Ok(LanguageCode::EN),
            "FR" => Ok(LanguageCode::FR),
            "IT" => Ok(LanguageCode::IT),
            "JA" => Ok(LanguageCode::JA),
            "SP" => Ok(LanguageCode::SP),
            _ => Err(FunctionalError::InvalidLanguageCode(s_ref.to_string())),
        }
    }

    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self::try_new(s).expect("invalid language code")
    }

    /// Locale segment used to build a Gatherer.wizards.com card URL.
    pub fn gatherer_locale(&self) -> &'static str {
        match self {
            LanguageCode::DE => "de-de",
            LanguageCode::EN => "en-us",
            LanguageCode::FR => "fr-fr",
            LanguageCode::IT => "it-it",
            LanguageCode::JA => "ja-jp",
            LanguageCode::SP => "es-es",
        }
    }
}

impl Display for LanguageCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LanguageCode::DE => write!(f, "DE"),
            LanguageCode::EN => write!(f, "EN"),
            LanguageCode::FR => write!(f, "FR"),
            LanguageCode::IT => write!(f, "IT"),
            LanguageCode::JA => write!(f, "JA"),
            LanguageCode::SP => write!(f, "SP"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_language_code_with_valid_fr_creates_instance() {
        let code = LanguageCode::new("FR");
        assert_eq!(code, LanguageCode::FR);
    }

    #[test]
    fn new_language_code_with_valid_en_creates_instance() {
        let code = LanguageCode::new("EN");
        assert_eq!(code, LanguageCode::EN);
    }

    #[test]
    fn new_language_code_with_valid_de_creates_instance() {
        let code = LanguageCode::new("DE");
        assert_eq!(code, LanguageCode::DE);
    }

    #[test]
    fn new_language_code_with_valid_it_creates_instance() {
        let code = LanguageCode::new("IT");
        assert_eq!(code, LanguageCode::IT);
    }

    #[test]
    fn new_language_code_with_valid_ja_creates_instance() {
        let code = LanguageCode::new("JA");
        assert_eq!(code, LanguageCode::JA);
    }

    #[test]
    fn new_language_code_with_valid_sp_creates_instance() {
        let code = LanguageCode::new("SP");
        assert_eq!(code, LanguageCode::SP);
    }

    #[test]
    fn try_new_language_code_with_invalid_code_returns_error() {
        let result = LanguageCode::try_new("XX");
        assert!(result.is_err());
    }

    #[test]
    fn try_new_language_code_with_invalid_code_contains_msg() {
        let result = LanguageCode::try_new("XX");
        match result {
            Err(FunctionalError::InvalidLanguageCode(msg)) => assert_eq!(msg, "XX"),
            _ => panic!("Expected InvalidLanguageCode variant"),
        }
    }

    #[test]
    fn new_language_code_is_case_insensitive() {
        let code_lower = LanguageCode::new("fr");
        let code_mixed = LanguageCode::new("Fr");
        assert_eq!(code_lower, LanguageCode::FR);
        assert_eq!(code_mixed, LanguageCode::FR);
    }

    #[test]
    fn display_language_code_returns_correct_string() {
        assert_eq!(LanguageCode::FR.to_string(), "FR");
        assert_eq!(LanguageCode::EN.to_string(), "EN");
        assert_eq!(LanguageCode::DE.to_string(), "DE");
        assert_eq!(LanguageCode::IT.to_string(), "IT");
        assert_eq!(LanguageCode::JA.to_string(), "JA");
        assert_eq!(LanguageCode::SP.to_string(), "SP");
    }

    #[test]
    fn gatherer_locale_returns_correct_locale_for_each_code() {
        assert_eq!(LanguageCode::EN.gatherer_locale(), "en-us");
        assert_eq!(LanguageCode::FR.gatherer_locale(), "fr-fr");
        assert_eq!(LanguageCode::DE.gatherer_locale(), "de-de");
        assert_eq!(LanguageCode::IT.gatherer_locale(), "it-it");
        assert_eq!(LanguageCode::JA.gatherer_locale(), "ja-jp");
        assert_eq!(LanguageCode::SP.gatherer_locale(), "es-es");
    }
}
