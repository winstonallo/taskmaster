use std::{
    fmt::Write,
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::Path,
};

use serde::{Deserialize, Deserializer};

fn get_random_string(len: usize) -> String {
    let mut buf = vec![0u8; len];
    let mut file = File::open("/dev/urandom").expect("could not open /dev/urandom");
    file.read_exact(&mut buf).expect("could not read from /dev/urandom");
    buf.iter().fold(String::from(""), |mut acc, byte| {
        write!(&mut acc, "{:02x}", byte).expect("failed to write");
        acc
    })
}

/// # `AccessibleDirectory`
/// `src/conf/proc/types/path.rs`
///
/// Implements the `serde::Deserializer` trait for the `workingdir` field of the configuration.
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

        let test_path = Path::new(&s).join(get_random_string(10));
        if !test_path.is_absolute() {
            return Err(serde::de::Error::custom("expected absolute path".to_string()));
        }
        match OpenOptions::new().write(true).create_new(true).open(&test_path) {
            Ok(file) => {
                drop(file);
                let _ = fs::remove_file(&test_path);
            }
            Err(err) => {
                return Err(serde::de::Error::custom(format!("'{s}' is not writable: {err}")));
            }
        }

        Ok(Self { path: s })
    }
}

/// # `ExecutableFile`
/// `src/conf/proc/types/path.rs`
///
/// Implements the `serde::Deserializer` trait for the `cmd` field of the configuration.
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
            return Err(serde::de::Error::custom("expected absolute path".to_string()));
        }

        Ok(Self { path: s })
    }
}

/// # `WritableFile`
/// `src/conf/proc/types/path.rs`
///
/// Implements the `serde::Deserializer` trait for the `stdout` and `stderr` fields of the configuration.
#[allow(unused)]
#[derive(Debug, Clone, Default)]
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

        match OpenOptions::new().write(true).create(true).truncate(false).open(&s) {
            Ok(file) => {
                drop(file);
                let _ = fs::remove_file(path);
            }
            Err(err) => return Err(serde::de::Error::custom(format!("'{s}' is not writable: {err}"))),
        }

        Ok(Self { path: s })
    }
}
