pub enum Encoding {
    ASCII,
    EBCDIC,
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::ASCII
    }
}

pub enum Framing {
    Unframed,
    MHeader,
    VHeader,
}

impl Default for Framing {
    fn default() -> Self {
        Framing::Unframed
    }
}

pub enum VariableLengthFormat {
    Symbolic,
    Byte,
}

impl Default for VariableLengthFormat {
    fn default() -> Self {
        VariableLengthFormat::Symbolic
    }
}

#[derive(Default)]
pub struct Codec {
    pub length_encoding: Encoding,
    pub data_encoding: Encoding,
    pub framing: Framing,
    pub ll_format: VariableLengthFormat,
}
