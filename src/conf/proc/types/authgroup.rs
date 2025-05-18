use serde::Deserialize;

#[derive(Clone)]
pub struct AuthGroup {
    id: u32,
    name: String,
}

impl AuthGroup {
    pub fn from_group_name(group_name: &str) -> Result<Self, String> {
        match get_group_id(&group_name) {
            Ok(id) => Ok(AuthGroup { id, name: group_name.into() }),
            Err(e) => Err(e.into()),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

fn get_group_id(group_name: &str) -> Result<u32, String> {
    let c_group = std::ffi::CString::new(group_name).map_err(|e| format!("{e}"))?;

    unsafe {
        let grp_ptr = libc::getgrnam(c_group.as_ptr());
        if grp_ptr.is_null() {
            Err(format!("Group '{group_name}' not found"))
        } else {
            Ok((*grp_ptr).gr_gid)
        }
    }
}

impl<'de> Deserialize<'de> for AuthGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let group_name = String::deserialize(deserializer)?;
        match AuthGroup::from_group_name(&group_name) {
            Ok(group) => Ok(group),
            Err(e) => Err(serde::de::Error::custom(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_group_id_success() {
        assert_eq!(get_group_id("root").unwrap(), 0);
    }

    #[test]
    fn get_group_id_nonexisting() {
        assert!(get_group_id("randomaaaahgroup").is_err());
    }
}
