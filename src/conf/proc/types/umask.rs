use serde::{Deserialize, Deserializer};

/// # `Umask`
/// `src/conf/proc/types/umask.rs`
///
/// Implements the `serde::Deserializer` trait for the `umask` field of the configuration.
#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct Umask {
    mask: u32,
}

impl Umask {
    pub fn mask(&self) -> u32 {
        self.mask
    }
}

impl Default for Umask {
    fn default() -> Self {
        Self { mask: 0o022 }
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

        // for c in s.chars() {
        //     match c {
        //         '0'..'7' => continue,
        //         _ => {
        //             return Err(serde::de::Error::custom(format!(
        //                 "invalid value for umask: {s}, expected 3 characters between '0' and '7'"
        //             )))
        //         }
        //     }
        // }

        let mask = match u32::from_str_radix(&s, 8) {
            Ok(mask) => mask,
            Err(err) => {
                return Err(serde::de::Error::custom(format!(
                    "invalid value for umask: '{s}', expected 3 octal digits: {err}"
                )));
            }
        };

        Ok(Self { mask })
    }
}
