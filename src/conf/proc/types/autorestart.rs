use serde::{Deserialize, Deserializer};

/// # `AutoRestart`
/// `src/conf/proc/types/path.rs`
///
/// Implements the `serde::Deserializer` trait for the `autorestart` field of the configuration.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct AutoRestart {
    mode: String,
    max_retries: Option<u8>,
}

#[allow(unused)]
impl AutoRestart {
    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn max_retries(&self) -> Option<u8> {
        self.max_retries
    }

    #[cfg(test)]
    pub fn test_autorestart() -> Self {
        Self {
            mode: String::from("no"),
            max_retries: None,
        }
    }
}

impl Default for AutoRestart {
    fn default() -> Self {
        Self {
            mode: String::from("no"),
            max_retries: None,
        }
    }
}

impl<'de> Deserialize<'de> for AutoRestart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "no" | "always" => Ok(Self { mode: s, max_retries: None }),
            _ if s.starts_with("on-failure[:") && s.ends_with("]") => {
                let max_retries_str = &s[12..s.len() - 1];
                let max_retries = match max_retries_str.parse::<u8>() {
                    Ok(n) => n,
                    Err(e) => {
                        return Err(serde::de::Error::custom(format!(
                            "invalid max-retries value for on-failure: {max_retries_str}: {e} (expected u8)"
                        )));
                    }
                };
                Ok(Self {
                    mode: String::from("on-failure"),
                    max_retries: Some(max_retries),
                })
            }
            _ => Err(serde::de::Error::custom(format!("invalid value for field 'autorestart': '{s}'"))),
        }
    }
}
