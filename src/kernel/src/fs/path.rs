use std::borrow::{Borrow, Cow};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use thiserror::Error;

/// A full path in the PS4 system.
#[repr(transparent)]
pub struct Vpath(str);

impl Vpath {
    pub fn new(data: &str) -> Option<&Self> {
        if Self::is_valid(data) {
            // SAFETY: This is ok because Vpath is #[repr(transparent)].
            Some(unsafe { &*(data as *const str as *const Vpath) })
        } else {
            None
        }
    }

    pub unsafe fn new_unchecked(data: &str) -> &Self {
        // SAFETY: This is ok because Vpath is #[repr(transparent)].
        &*(data as *const str as *const Vpath)
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

impl From<&Vpath> for String {
    fn from(value: &Vpath) -> Self {
        String::from(&value.0)
    }
}

impl<'a> TryFrom<&'a str> for &'a Vpath {
    type Error = ();

    fn try_from(value: &'a str) -> Result<&'a Vpath, Self::Error> {
        Vpath::new(value).ok_or(())
    }
}

impl Display for Vpath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The owned version of [`Vpath`].
#[derive(PartialEq, Eq, Hash)]
pub struct VpathBuf(Cow<'static, str>);

impl VpathBuf {
    pub const fn new() -> Self {
        Self(Cow::Borrowed("/"))
    }

    pub fn push(&mut self, component: &str) -> Result<(), PushError> {
        match component {
            "" => Err(PushError::EmptyComponent),
            "." | ".." => Err(PushError::ForbiddenComponent),
            v => {
                let data = self.0.to_mut();

                if data.len() != 1 {
                    data.push('/');
                }

                data.push_str(v);
                Ok(())
            }
        }
    }
}

impl From<&Vpath> for VpathBuf {
    fn from(value: &Vpath) -> Self {
        Self(Cow::Owned(value.into()))
    }
}

impl TryFrom<&'static str> for VpathBuf {
    type Error = ();

    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        if Vpath::is_valid(value) {
            Ok(Self(Cow::Borrowed(value)))
        } else {
            Err(())
        }
    }
}

impl TryFrom<String> for VpathBuf {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if Vpath::is_valid(&value) {
            Ok(Self(Cow::Owned(value)))
        } else {
            Err(())
        }
    }
}

impl Deref for VpathBuf {
    type Target = Vpath;

    fn deref(&self) -> &Vpath {
        unsafe { Vpath::new_unchecked(self.0.borrow()) }
    }
}

impl Borrow<Vpath> for VpathBuf {
    fn borrow(&self) -> &Vpath {
        self.deref()
    }
}

impl Display for VpathBuf {
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

/// Represents the errors for [`VpathBuf::push()`].
#[derive(Debug, Error)]
pub enum PushError {
    #[error("the component is empty")]
    EmptyComponent,

    #[error("the component is forbidden")]
    ForbiddenComponent,
}
