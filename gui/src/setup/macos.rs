use core_foundation::base::TCFType;
use core_foundation::propertylist::{CFPropertyList, CFPropertyListSubClass};
use core_foundation::string::CFString;
use core_foundation_sys::preferences::{
    CFPreferencesAppSynchronize, CFPreferencesCopyAppValue, CFPreferencesSetAppValue,
    kCFPreferencesCurrentApplication,
};
use thiserror::Error;

pub fn read_data_root() -> Result<Option<String>, DataRootError> {
    // Read value.
    let val = KEY.with(|k| unsafe {
        CFPreferencesCopyAppValue(k.as_concrete_TypeRef(), kCFPreferencesCurrentApplication)
    });

    if val.is_null() {
        return Ok(None);
    }

    // Convert value.
    let val = unsafe { CFPropertyList::wrap_under_create_rule(val) };

    val.downcast_into::<CFString>()
        .ok_or_else(|| KEY.with(|k| DataRootError::InvalidPreferenceValue(k.to_string())))
        .map(|v| Some(v.to_string()))
}

pub fn write_data_root(path: impl AsRef<str>) -> Result<(), DataRootError> {
    // Write value.
    let v = CFString::new(path.as_ref()).into_CFPropertyList();

    KEY.with(|k| unsafe {
        CFPreferencesSetAppValue(
            k.as_concrete_TypeRef(),
            v.as_concrete_TypeRef(),
            kCFPreferencesCurrentApplication,
        )
    });

    // Writes to permanent storage.
    if unsafe { CFPreferencesAppSynchronize(kCFPreferencesCurrentApplication) == 0 } {
        return Err(DataRootError::SynchronizePreferences);
    }

    Ok(())
}

/// Represents an error when read or write data root fails.
#[derive(Debug, Error)]
pub enum DataRootError {
    #[error("invalid value for preference {0}")]
    InvalidPreferenceValue(String),

    #[error("couldn't synchronize preferences")]
    SynchronizePreferences,
}

thread_local! {
    static KEY: CFString = CFString::from_static_string("DataRoot");
}
