use std::borrow::{Borrow, Cow};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use thiserror::Error;

/// See `devfs_pathpath` on the PS4 for a reference.
pub fn path_contains(p1: &str, p2: &str) -> bool {
    let mut p1 = p1.bytes();
    let mut p2 = p2.bytes();

    loop {
        match (p1.next(), p2.next()) {
            (None, None) => break true,
            (None, Some(_)) => break false,
            (Some(p1), None) => break p1 == b'/',
            (Some(p1), Some(p2)) => {
                if p1 == p2 {
                    continue;
                } else {
                    break false;
                }
            }
        }
    }
}

/// A full path in the PS4 system.
#[derive(PartialEq, Eq, Hash)]
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

    pub const unsafe fn new_unchecked(data: &str) -> &Self {
        // SAFETY: This is ok because VPath is #[repr(transparent)].
        &*(data as *const str as *const VPath)
    }

    pub fn is_absolute(&self) -> bool {
        self.0.starts_with('/')
    }

    pub fn join<C: AsRef<str>>(&self, component: C) -> Result<VPathBuf, ComponentError> {
        let mut r = self.to_owned();
        r.push(component)?;
        Ok(r)
    }

    /// Gets the parent path.
    pub fn parent(&self) -> Option<&Self> {
        if self.0.len() == 1 {
            // This path is a root directory ("/").
            None
        } else {
            let end = self.0.rfind('/').unwrap();
            let data = if end == 0 { "/" } else { &self.0[..end] };

            // SAFETY: This is safe because the data is still a valid path when the last component
            // is removed (e.g. "/abc/def" => "/abc").
            Some(unsafe { Self::new_unchecked(data) })
        }
    }

    pub fn file_name(&self) -> Option<&str> {
        if self.0.len() == 1 {
            // This path is a root directory ("/").
            None
        } else {
            let sep = self.0.rfind('/').unwrap();
            Some(&self.0[(sep + 1)..])
        }
    }

    pub fn components(&self) -> Components<'_> {
        // SAFETY: The path always is an absolute path that mean we have at least / in the
        // beginning.
        Components(unsafe { self.0.get_unchecked(1..) })
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

impl Deref for VPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<VPath> for VPath {
    fn as_ref(&self) -> &VPath {
        self
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
        VPathBuf(Cow::Owned(self.0.to_owned()))
    }
}

impl Display for VPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&VPath> for String {
    fn from(v: &VPath) -> Self {
        v.0.to_owned()
    }
}

/// The owned version of [`VPath`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VPathBuf(Cow<'static, str>);

impl VPathBuf {
    pub const fn new() -> Self {
        Self(Cow::Borrowed("/"))
    }

    pub fn push(&mut self, component: impl AsRef<str>) -> Result<(), ComponentError> {
        // Check if component valid.
        let v = match component.as_ref() {
            "" => return Err(ComponentError::Empty),
            "." | ".." => return Err(ComponentError::Forbidden),
            v => {
                if v.contains('/') {
                    return Err(ComponentError::HasPathSeparator);
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

    pub fn set_extension(&mut self, ext: &str) -> Result<(), SetExtensionError> {
        // Check extension.
        if ext.contains('/') {
            return Err(SetExtensionError::Invalid);
        }

        // Check if root directory.
        let s = self.0.to_mut();

        if s.len() == 1 {
            return Err(SetExtensionError::PathIsRoot);
        }

        // Find the last ".".
        let i = match s.rfind('.') {
            Some(v) if v > 0 => v,
            _ => s.len(),
        };

        // Check if we need to remove extension instead.
        if ext.is_empty() {
            s.replace_range(i.., "");
        } else {
            s.replace_range(i.., &format!(".{ext}"));
        }

        Ok(())
    }
}

impl Deref for VPathBuf {
    type Target = VPath;

    fn deref(&self) -> &VPath {
        self.borrow()
    }
}

impl From<&VPath> for VPathBuf {
    fn from(value: &VPath) -> Self {
        value.to_owned()
    }
}

impl TryFrom<&str> for VPathBuf {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if VPath::is_valid(value) {
            Ok(Self(Cow::Owned(value.to_owned())))
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

impl AsRef<VPath> for VPathBuf {
    fn as_ref(&self) -> &VPath {
        self.borrow()
    }
}

impl Borrow<VPath> for VPathBuf {
    fn borrow(&self) -> &VPath {
        // SAFETY: This is safe because VPathBuf has the same check as VPath.
        unsafe { VPath::new_unchecked(self.0.borrow()) }
    }
}

impl PartialEq<VPath> for VPathBuf {
    fn eq(&self, other: &VPath) -> bool {
        self.0 == other.0
    }
}

impl Display for VPathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<VPathBuf> for String {
    fn from(v: VPathBuf) -> Self {
        v.0.into_owned()
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

/// Represents an error for path component.
#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("the component is empty")]
    Empty,

    #[error("the component is forbidden")]
    Forbidden,

    #[error("the component contains path separator")]
    HasPathSeparator,
}

/// Error of [`VPathBuf::set_extension()`].
#[derive(Debug, Error)]
pub enum SetExtensionError {
    #[error("extension is not valid")]
    Invalid,

    #[error("path is a root directory")]
    PathIsRoot,
}
