use std::borrow::{Borrow, Cow};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use thiserror::Error;

/// A full path in the PS4 system.
#[repr(transparent)]
pub struct VPath(str);

impl VPath {
    pub fn new(data: &str) -> Option<&Self> {
        if Self::is_valid(data) {
            // SAFETY: This is ok because VPath is #[repr(transparent)].
            Some(unsafe { &*(data as *const str as *const VPath) })
        } else {
            None
        }
    }

    pub unsafe fn new_unchecked(data: &str) -> &Self {
        // SAFETY: This is ok because VPath is #[repr(transparent)].
        &*(data as *const str as *const VPath)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Gets the parent path.
    pub fn parent(&self) -> Option<&Self> {
        if self.0.len() == 1 {
            // This path is a root directory ("/").
            None
        } else {
            // SAFETY: We already forced the path to be an absolute path so that mean it will have
            // at least one / in the beginning.
            let end = unsafe { self.0.rfind('/').unwrap_unchecked() };
            let data = if end == 0 { "/" } else { &self.0[..end] };

            Some(unsafe { Self::new_unchecked(data) })
        }
    }

    pub fn components(&self) -> Components<'_> {
        // SAFETY: The path always is an absolute path that mean we have at least / in the
        // beginning.
        Components(unsafe { &self.0.get_unchecked(1..) })
    }

    fn is_valid(data: &str) -> bool {
        // Do a simple check first.
        if data.is_empty() || !data.starts_with('/') || data.ends_with('/') {
            return false;
        }

        // Check thoroughly.
        let mut sep = 0;

        for (i, ch) in data.bytes().enumerate() {
            if i == 0 || ch != b'/' {
                continue;
            }

            // Disallow a consecutive of the separator, "." and "..".
            let com = &data[(sep + 1)..i];

            if com.is_empty() || com == "." || com == ".." {
                return false;
            }

            sep = i;
        }

        true
    }
}

impl From<&VPath> for String {
    fn from(value: &VPath) -> Self {
        String::from(&value.0)
    }
}

impl<'a> TryFrom<&'a str> for &'a VPath {
    type Error = ();

    fn try_from(value: &'a str) -> Result<&'a VPath, Self::Error> {
        VPath::new(value).ok_or(())
    }
}

impl ToOwned for VPath {
    type Owned = VPathBuf;

    fn to_owned(&self) -> Self::Owned {
        self.into()
    }
}

impl Display for VPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The owned version of [`VPath`].
#[derive(PartialEq, Eq, Hash)]
pub struct VPathBuf(Cow<'static, str>);

impl VPathBuf {
    pub const fn new() -> Self {
        Self(Cow::Borrowed("/"))
    }

    pub fn push(&mut self, component: &str) -> Result<(), PushError> {
        // Check if component valid.
        let v = match component {
            "" => return Err(PushError::Empty),
            "." | ".." => return Err(PushError::Forbidden),
            v => {
                if v.contains('/') {
                    return Err(PushError::HasPathSeparator);
                } else {
                    v
                }
            }
        };

        // Append.
        let data = self.0.to_mut();

        if data.len() != 1 {
            data.push('/');
        }

        data.push_str(v);
        Ok(())
    }
}

impl From<&VPath> for VPathBuf {
    fn from(value: &VPath) -> Self {
        Self(Cow::Owned(value.into()))
    }
}

impl TryFrom<&'static str> for VPathBuf {
    type Error = ();

    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        if VPath::is_valid(value) {
            Ok(Self(Cow::Borrowed(value)))
        } else {
            Err(())
        }
    }
}

impl TryFrom<String> for VPathBuf {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if VPath::is_valid(&value) {
            Ok(Self(Cow::Owned(value)))
        } else {
            Err(())
        }
    }
}

impl Deref for VPathBuf {
    type Target = VPath;

    fn deref(&self) -> &VPath {
        unsafe { VPath::new_unchecked(self.0.borrow()) }
    }
}

impl Borrow<VPath> for VPathBuf {
    fn borrow(&self) -> &VPath {
        self.deref()
    }
}

impl Display for VPathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.borrow())
    }
}

/// An iterator over the path components.
pub struct Components<'a>(&'a str);

impl<'a> Iterator for Components<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if no more components available.
        if self.0.is_empty() {
            return None;
        }

        // Get next component.
        let end = self.0.find('/').unwrap_or(self.0.len());
        let component = &self.0[..end];

        // Advance.
        self.0 = if end == self.0.len() {
            &self.0[end..]
        } else {
            &self.0[(end + 1)..]
        };

        Some(component)
    }
}

/// Represents the errors for [`VPathBuf::push()`].
#[derive(Debug, Error)]
pub enum PushError {
    #[error("the component is empty")]
    Empty,

    #[error("the component is forbidden")]
    Forbidden,

    #[error("the component contains path separator")]
    HasPathSeparator,
}
