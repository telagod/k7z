use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct SecretString(Zeroizing<String>);

impl SecretString {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(Zeroizing::new(raw.into()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_round_trip() {
        let secret = SecretString::new("p@ss");
        assert_eq!(secret.as_str(), "p@ss");
    }
}
