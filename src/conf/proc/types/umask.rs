use serde::{Deserialize, Deserializer};

/// # `Umask`
/// `src/conf/proc/types/umask.rs`
///
/// Implements the `serde::Deserializer` trait for the `umask` field of the configuration.
#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct Umask {
    mask: String,
}

impl Umask {
    pub fn mask(&self) -> &str {
        &self.mask
    }
}

impl Default for Umask {
    fn default() -> Self {
        Self { mask: String::from("022") }
    }
}

impl<'de> Deserialize<'de> for Umask {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.len() != 3 {
            return Err(serde::de::Error::custom(format!("invalid length for umask, expected 3, got {}", s.len())));
        }

        for c in s.chars() {
            match c {
                '0'..'7' => continue,
                _ => {
                    return Err(serde::de::Error::custom(format!(
                        "invalid value for umask: {s}, expected 3 characters between '0' and '7'"
                    )))
                }
            }
        }

        Ok(Self { mask: s })
    }
}
