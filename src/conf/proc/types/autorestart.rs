use serde::{Deserialize, Deserializer};

use crate::{log_info, proc_info};

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
    /// Retrieves the value set in the config for `autorestart`.
    ///
    /// Possible values: `no`, `always`, `on-failure`.
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// Retrieves the max-retries value set in the config for on-failure.
    ///
    /// ## Panics
    /// Panics if the retry mode is anything else than `on-failure`,
    /// due to it being the only case where `max-retries` is set.   
    pub fn max_retries(&self) -> u8 {
        self.max_retries.expect("this method should only be called after checking the autorestart mode")
    }
}

impl Default for AutoRestart {
    fn default() -> Self {
        Self {
            mode: "no".to_string(),
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

        if s.as_str() == "no" || s.as_str() == "always" {
            return Ok(Self { mode: s, max_retries: None });
        }

        if !s.starts_with("on-failure[:") || !s.ends_with("]") {
            return Err(serde::de::Error::custom(format!(
                "invalid value for on-failure: {s}, expected 'on-failure[:max-retries]'"
            )));
        }

        let max_retries_str = &s[12..s.len() - 1];

        let max_retries = match max_retries_str.parse::<u8>() {
            Ok(n) => n,
            Err(e) => {
                return Err(serde::de::Error::custom(format!(
                    "invalid max-retries value for on-failure: {max_retries_str}: {e}, expected u8"
                )));
            }
        };

        Ok(Self {
            mode: "on-failure".to_string(),
            max_retries: Some(max_retries),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn max_retries() {
        let _ = AutoRestart {
            mode: "no".to_string(),
            max_retries: None,
        }
        .max_retries();
    }
}
