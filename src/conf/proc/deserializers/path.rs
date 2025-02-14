use std::{
    fs::{self, OpenOptions},
    os::unix::fs::PermissionsExt,
    path::Path,
};

use serde::{Deserialize, Deserializer};

/// `serde` Deserializer checking whether the path is a directory,
/// accessible for writing by the process.
#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibleDirectory {
    path: String,
}

#[allow(unused)]
impl AccessibleDirectory {
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Default for AccessibleDirectory {
    fn default() -> Self {
        Self { path: String::from("/tmp") }
    }
}

impl<'de> Deserialize<'de> for AccessibleDirectory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let md = match fs::metadata(s.clone()) {
            Ok(md) => md,
            Err(err) => {
                return Err(serde::de::Error::custom(format!("expected path to directory with write permissions: {err}")));
            }
        };
        if !md.is_dir() {
            return Err(serde::de::Error::custom(format!("'{s}' is not a directory")));
        }

        let test_path = Path::new(&s).join(".write_test");
        if !test_path.is_absolute() {
            return Err(serde::de::Error::custom(format!("expected absolute path")));
        }
        match OpenOptions::new().write(true).create_new(true).open(&test_path) {
            Ok(file) => {
                drop(file);
                let _ = fs::remove_file(&test_path);
            }
            Err(err) => return Err(serde::de::Error::custom(format!("'{s}' is not writable: {err}"))),
        }

        Ok(Self { path: s })
    }
}

/// `serde` Deserializer checking whether the path is an executable,
/// accessible for executing by the process.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ExecutableFile {
    path: String,
}

#[allow(unused)]
impl ExecutableFile {
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Default for ExecutableFile {
    fn default() -> Self {
        Self { path: String::from("/tmp") }
    }
}

impl<'de> Deserialize<'de> for ExecutableFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let md = match fs::metadata(s.clone()) {
            Ok(md) => md,
            Err(err) => {
                return Err(serde::de::Error::custom(format!("expected path to file with execute permissions: {err}")));
            }
        };
        if !md.is_file() {
            return Err(serde::de::Error::custom(format!("'{s}' is not a file")));
        }

        if md.permissions().mode() & 0o111 == 0 {
            return Err(serde::de::Error::custom(format!("'{s}' is not executable")));
        }

        let test_path = Path::new(&s).join(".write_test");
        if !test_path.is_absolute() {
            return Err(serde::de::Error::custom(format!("expected absolute path")));
        }

        Ok(Self { path: s })
    }
}

/// `serde` Deserializer checking whether the path is writable by the process.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct WritableFile {
    path: String,
}

#[allow(unused)]
impl WritableFile {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn from_path(path: &str) -> Self {
        Self { path: String::from(path) }
    }
}

impl Default for WritableFile {
    fn default() -> Self {
        Self { path: String::from("") }
    }
}

impl<'de> Deserialize<'de> for WritableFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let path = Path::new(&s);

        if path.exists() {
            let md = match fs::metadata(&s) {
                Ok(md) => md,
                Err(err) => return Err(serde::de::Error::custom(format!("failed to get metadata for '{s}': {err}"))),
            };
            if !md.is_file() {
                return Err(serde::de::Error::custom(format!("'{s}' exists but is not a file")));
            }
        }

        match OpenOptions::new().write(true).create(true).open(&s) {
            Ok(file) => {
                drop(file);
                let _ = fs::remove_file(path);
            }
            Err(err) => return Err(serde::de::Error::custom(format!("'{s}' is not writable: {err}"))),
        }

        Ok(Self { path: s })
    }
}
