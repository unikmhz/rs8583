use bytes::{BufMut, BytesMut};
use encoding8::ascii;

use crate::error::RS8583Error;

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
    length_encoding: Encoding,
    data_encoding: Encoding,
    framing: Framing,
    ll_format: VariableLengthFormat,
}

impl Codec {
    pub fn length_size_bytes(&self, len: usize) -> usize {
        match self.ll_format {
            VariableLengthFormat::Symbolic => len,
            VariableLengthFormat::Byte => 1,
        }
    }

    pub fn byte_to_length(&self, len_byte: u8) -> Result<usize, RS8583Error> {
        if let VariableLengthFormat::Byte = self.ll_format {
            return Ok(len_byte as usize);
        }
        let offset: u8 = match self.length_encoding {
            Encoding::ASCII => 0x30,
            Encoding::EBCDIC => 0xf0,
        };
        match len_byte {
            n if n > (offset + 9) => Err(RS8583Error::parse_error(format!(
                "Length byte out of range: 0x{:02x}",
                n
            ))),
            n if n < offset => Err(RS8583Error::parse_error(format!(
                "Length byte out of range: 0x{:02x}",
                n
            ))),
            n => Ok((n - offset) as usize),
        }
    }

    pub fn serialize_prefix(&self, buf: &mut BytesMut, prefix_len: usize, data_len: usize) -> Result<(), RS8583Error> {
        match self.ll_format {
            VariableLengthFormat::Byte => {
                if data_len > (std::u8::MAX as usize) {
                    Err(RS8583Error::parse_error(format!(
                        "Length out of range: {}",
                        data_len
                    )))
                } else {
                    buf.put_u8(data_len as u8);
                    Ok(())
                }
            }
            VariableLengthFormat::Symbolic => {
                // TODO: efficiency
                let mut prefix = format!("{0:01$}", data_len, prefix_len).into_bytes();
                if let Encoding::EBCDIC = self.length_encoding {
                    for ch in prefix.iter_mut() {
                        *ch = ascii::to_ebcdic(*ch);
                    }
                }
                buf.extend_from_slice(&prefix);
                Ok(())
            }
        }
    }
}
