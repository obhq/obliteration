/// Flags that control kernel behavior related to QA.
///
/// See https://www.psdevwiki.com/ps4/QA_Flagging for a list of known flags.
#[repr(C)]
#[derive(Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct QaFlags(#[cfg_attr(feature = "serde", serde(with = "serde_bytes"))] [u8; 16]);

impl QaFlags {
    pub fn internal_dev(&self) -> bool {
        (self.0[0] & 4) != 0
    }
}
