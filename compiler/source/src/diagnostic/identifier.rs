//! Compiler rules and tool-owned identifiers share transport, not a registry.
use super::{DiagnosticCategory, DiagnosticCode, DiagnosticRole};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticIdentifier {
    Rule(DiagnosticCode),
    External(String),
}

impl DiagnosticIdentifier {
    pub fn parse(value: &str) -> Option<Self> {
        if let Some(code) = DiagnosticCode::parse(value) {
            return Some(Self::Rule(code));
        }
        let bytes = value.as_bytes();
        (!bytes.is_empty()
            && bytes.len() <= 128
            && bytes[0].is_ascii_uppercase()
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b'_'))
        .then(|| Self::External(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Rule(code) => code.as_str(),
            Self::External(code) => code,
        }
    }

    pub fn category(&self) -> Option<DiagnosticCategory> {
        match self {
            Self::Rule(code) => Some(code.category()),
            Self::External(_) => None,
        }
    }

    pub fn role(&self) -> DiagnosticRole {
        match self {
            Self::Rule(code) => code.role(),
            Self::External(_) => DiagnosticRole::Note,
        }
    }
}

impl From<DiagnosticCode> for DiagnosticIdentifier {
    fn from(code: DiagnosticCode) -> Self {
        Self::Rule(code)
    }
}
impl From<&DiagnosticIdentifier> for DiagnosticIdentifier {
    fn from(code: &DiagnosticIdentifier) -> Self {
        code.clone()
    }
}
impl PartialEq<DiagnosticCode> for DiagnosticIdentifier {
    fn eq(&self, other: &DiagnosticCode) -> bool {
        matches!(self, Self::Rule(code) if code == other)
    }
}
impl PartialEq<DiagnosticCode> for &DiagnosticIdentifier {
    fn eq(&self, other: &DiagnosticCode) -> bool {
        (*self).eq(other)
    }
}
impl std::fmt::Display for DiagnosticIdentifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
impl serde::Serialize for DiagnosticIdentifier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> serde::Deserialize<'de> for DiagnosticIdentifier {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| serde::de::Error::custom("invalid diagnostic identifier"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_identifiers_keep_their_spelling_without_becoming_scoop_rules() {
        let code = DiagnosticIdentifier::parse("PEER_WARNING").unwrap();
        assert_eq!(code.as_str(), "PEER_WARNING");
        assert_eq!(code.category(), None);
        assert_eq!(DiagnosticCode::parse(code.as_str()), None);
        assert!(DiagnosticIdentifier::parse("peer warning").is_none());
        assert_eq!(
            DiagnosticIdentifier::parse("E0501").unwrap(),
            DiagnosticCode::E0501
        );
    }
}
