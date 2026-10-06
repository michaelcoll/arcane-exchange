use crate::domain::error::FunctionalError;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetCode(String);

impl SetCode {
    pub fn try_new(s: impl Into<String>) -> Result<Self, FunctionalError> {
        let name = s.into().to_uppercase();
        if name.chars().count() >= 3 && name.chars().count() <= 5 {
            Ok(SetCode(name))
        } else {
            Err(FunctionalError::InvalidSetCode(format!(
                "set code must be between 3 and 5 characters (got {})",
                name
            )))
        }
    }

    pub fn new(s: impl Into<String>) -> Self {
        Self::try_new(s).expect("invalid set code")
    }
}

impl Display for SetCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SetCode {
    fn from(s: &str) -> Self {
        SetCode::new(s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetName {
    pub code: SetCode,
    pub name: String,
}

impl SetName {
    #[allow(dead_code)]
    pub fn new(code: impl Into<SetCode>, name: impl Into<String>) -> Self {
        SetName {
            code: code.into(),
            name: name.into().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_set_code_with_valid_length_creates_instance() {
        let code = SetCode::new("ABC");
        assert_eq!(code.0, "ABC");
    }

    #[test]
    fn new_set_code_with_invalid_length_returns_error() {
        let result = SetCode::try_new("AB");
        assert!(result.is_err());
    }

    #[test]
    fn new_set_code_with_invalid_length_contains_message() {
        let result = SetCode::try_new("AB");
        match result {
            Err(FunctionalError::InvalidSetCode(msg)) => {
                assert!(msg.contains("set code must be between 3 and 5 characters"))
            }
            _ => panic!("Expected InvalidSetCode variant"),
        }
    }

    #[test]
    fn new_set_code_with_too_long_returns_error() {
        let result = SetCode::try_new("ABCDE1");
        assert!(result.is_err());
    }

    #[test]
    fn display_set_code_returns_correct_string() {
        let code = SetCode::new("XYZ");
        assert_eq!(code.to_string(), "XYZ");
    }

    #[test]
    fn new_set_name_creates_instance_with_correct_values() {
        let code = SetCode::new("DEF");
        let name = "Set Name";
        let set_name = SetName::new(code.clone(), name);
        assert_eq!(set_name.code, code);
        assert_eq!(set_name.name, name);
    }

    #[test]
    fn set_name_equality_works_correctly() {
        let code1 = SetCode::new("GHI");
        let code2 = SetCode::new("JKL");
        let set_name1 = SetName::new(code1.clone(), "Name1");
        let set_name2 = SetName::new(code1, "Name1");
        let set_name3 = SetName::new(code2, "Name2");

        assert_eq!(set_name1, set_name2);
        assert_ne!(set_name1, set_name3);
    }
}
