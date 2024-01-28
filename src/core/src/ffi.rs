/// Represents a QString on the C++ side.
pub struct QString(());

impl QString {
    pub fn set(&mut self, v: impl AsRef<str>) {
        let v = v.as_ref();
        unsafe { qstring_set(self, v.as_ptr(), v.len()) };
    }
}

#[allow(improper_ctypes)]
extern "C" {
    fn qstring_set(s: &mut QString, v: *const u8, l: usize);
}
