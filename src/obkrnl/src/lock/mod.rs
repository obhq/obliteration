pub use self::gutex::*;
pub use self::mutex::*;

mod gutex;
mod mutex;

const MTX_UNOWNED: usize = 4;
