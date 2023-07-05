/// Outputs of the syscall.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Output {
    pub rax: usize,
    pub rdx: usize,
}

impl Output {
    pub const ZERO: Output = Output { rax: 0, rdx: 0 };
}

impl From<usize> for Output {
    fn from(value: usize) -> Self {
        Self { rax: value, rdx: 0 }
    }
}
