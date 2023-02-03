use std::path::{Path, PathBuf};
use util::mem::{read_u32_be, write_u32_be};

pub struct Entry {
    id: u32,
    filename_offset: u32,
    flags1: u32,
    flags2: u32,
    data_offset: u32,
    data_size: u32,
}

impl Entry {
    pub const RAW_SIZE: usize = 32;

    pub const ENTRY_KEYS: u32 = 0x00000010;
    pub const PFS_IMAGE_KEY: u32 = 0x00000020;
    pub const NPTITLE_DAT: u32 = 0x00000402;
    pub const NPBIND_DAT: u32 = 0x00000403;
    pub const SELFINFO_DAT: u32 = 0x00000404;
    pub const IMAGEINFO_DAT: u32 = 0x00000406;
    pub const TARGET_DELTAINFO_DAT: u32 = 0x00000407;
    pub const ORIGIN_DELTAINFO_DAT: u32 = 0x00000408;
    pub const PARAM_SFO: u32 = 0x00001000;
    pub const PRONUNCIATION_XML: u32 = 0x00001004;
    pub const PRONUNCIATION_SIG: u32 = 0x00001005;
    pub const PIC1_PNG: u32 = 0x00001006;
    pub const PUBTOOLINFO_DAT: u32 = 0x00001007;
    pub const APP_PLAYGO_CHUNK_DAT: u32 = 0x00001008;
    pub const APP_PLAYGO_CHUNK_SHA: u32 = 0x00001009;
    pub const APP_PLAYGO_MANIFEST_XML: u32 = 0x0000100a;
    pub const SHAREPARAM_JSON: u32 = 0x0000100b;
    pub const SHAREOVERLAYIMAGE_PNG: u32 = 0x0000100c;
    pub const SAVE_DATA_PNG: u32 = 0x0000100d;
    pub const SHAREPRIVACYGUARDIMAGE_PNG: u32 = 0x0000100e;
    pub const ICON0_PNG: u32 = 0x00001200;
    pub const PIC0_PNG: u32 = 0x00001220;
    pub const SND0_AT9: u32 = 0x00001240;
    pub const CHANGEINFO_CHANGEINFO_XML: u32 = 0x00001260;
    pub const ICON0_DDS: u32 = 0x00001280;
    pub const PIC0_DDS: u32 = 0x000012a0;
    pub const PIC1_DDS: u32 = 0x000012c0;

    pub fn read(raw: *const u8) -> Self {
        let id = read_u32_be(raw, 0);
        let filename_offset = read_u32_be(raw, 4);
        let flags1 = read_u32_be(raw, 8);
        let flags2 = read_u32_be(raw, 12);
        let data_offset = read_u32_be(raw, 16);
        let data_size = read_u32_be(raw, 20);

        Self {
            id,
            filename_offset,
            flags1,
            flags2,
            data_offset,
            data_size,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn is_encrypted(&self) -> bool {
        self.flags1 & 0x80000000 != 0
    }

    pub fn key_index(&self) -> usize {
        ((self.flags2 & 0xf000) >> 12) as _
    }

    pub fn data_offset(&self) -> usize {
        self.data_offset as _
    }

    pub fn data_size(&self) -> usize {
        self.data_size as _
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        let p = buf.as_mut_ptr();

        write_u32_be(p, 0, self.id);
        write_u32_be(p, 4, self.filename_offset);
        write_u32_be(p, 8, self.flags1);
        write_u32_be(p, 12, self.flags2);
        write_u32_be(p, 16, self.data_offset);
        write_u32_be(p, 20, self.data_size);

        buf
    }

    pub fn to_path<B: AsRef<Path>>(&self, base: B) -> Option<PathBuf> {
        let base = base.as_ref();
        let path = match self.id {
            Self::NPTITLE_DAT => base.join("nptitle.dat"),
            Self::NPBIND_DAT => base.join("npbind.dat"),
            Self::SELFINFO_DAT => base.join("selfinfo.dat"),
            Self::IMAGEINFO_DAT => base.join("imageinfo.dat"),
            Self::TARGET_DELTAINFO_DAT => base.join("target-deltainfo.dat"),
            Self::ORIGIN_DELTAINFO_DAT => base.join("origin-deltainfo.dat"),
            Self::PARAM_SFO => base.join("param.sfo"),
            Self::PRONUNCIATION_XML => base.join("pronunciation.xml"),
            Self::PRONUNCIATION_SIG => base.join("pronunciation.sig"),
            Self::PIC1_PNG => base.join("pic1.png"),
            Self::PUBTOOLINFO_DAT => base.join("pubtoolinfo.dat"),
            Self::APP_PLAYGO_CHUNK_DAT => base.join("app").join("playgo-chunk.dat"),
            Self::APP_PLAYGO_CHUNK_SHA => base.join("app").join("playgo-chunk.sha"),
            Self::APP_PLAYGO_MANIFEST_XML => base.join("app").join("playgo-manifest.xml"),
            Self::SHAREPARAM_JSON => base.join("shareparam.json"),
            Self::SHAREOVERLAYIMAGE_PNG => base.join("shareoverlayimage.png"),
            Self::SAVE_DATA_PNG => base.join("save_data.png"),
            Self::SHAREPRIVACYGUARDIMAGE_PNG => base.join("shareprivacyguardimage.png"),
            Self::ICON0_PNG => base.join("icon0.png"),
            Self::PIC0_PNG => base.join("pic0.png"),
            Self::SND0_AT9 => base.join("snd0.at9"),
            Self::CHANGEINFO_CHANGEINFO_XML => base.join("changeinfo").join("changeinfo.xml"),
            Self::ICON0_DDS => base.join("icon0.dds"),
            Self::PIC0_DDS => base.join("pic0.dds"),
            Self::PIC1_DDS => base.join("pic1.dds"),
            _ => return None,
        };

        Some(path)
    }
}
