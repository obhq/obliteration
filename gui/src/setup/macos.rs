use core_foundation::base::TCFType;
use core_foundation::propertylist::CFPropertyList;
use core_foundation::string::CFString;
use core_foundation_sys::preferences::{
    kCFPreferencesCurrentApplication, CFPreferencesCopyAppValue,
};
use thiserror::Error;

pub fn read_data_root() -> Result<Option<String>, DataRootError> {
    // Read value.
    let key = CFString::from_static_string("DataRoot");
    let val = unsafe {
        CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), kCFPreferencesCurrentApplication)
    };

    if val.is_null() {
        return Ok(None);
    }

    // Convert value.
    let val = unsafe { CFPropertyList::wrap_under_create_rule(val) };

    val.downcast_into::<CFString>()
        .ok_or_else(|| DataRootError::InvalidPreferenceValue(key.to_string()))
        .map(|v| Some(v.to_string()))
}

pub fn write_data_root(path: impl AsRef<str>) -> Result<(), DataRootError> {
    todo!()
}

/// Represents an error when read or write data root fails.
#[derive(Debug, Error)]
pub enum DataRootError {
    #[error("invalid value for preference {0}")]
    InvalidPreferenceValue(String),
}
