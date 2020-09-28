use crate::error::RS8583Error;
use bytes::{Buf, Bytes, BytesMut};
use std::cmp::min;

use crate::codec::Codec;
use crate::field::Field;

pub enum FieldType {
    A,
    N,
    S,
    NS,
    AN,
    ANS,
    B,
}

impl Default for FieldType {
    fn default() -> Self {
        FieldType::ANS
    }
}

pub enum LengthType {
    Fixed,
    LVar,
    LLVar,
    LLLVar,
    LLLLVar,
    BitMap,
}

impl LengthType {
    pub fn length_size(&self) -> usize {
        match self {
            Self::LVar => 1,
            Self::LLVar => 2,
            Self::LLLVar => 3,
            Self::LLLLVar => 4,
            _ => 0,
        }
    }
}

impl Default for LengthType {
    fn default() -> Self {
        LengthType::Fixed
    }
}

pub enum SensitivityType {
    Normal,
    MaskPAN,
    MaskAll,
}

impl Default for SensitivityType {
    fn default() -> Self {
        SensitivityType::Normal
    }
}

#[derive(Default)]
pub struct FieldSpec {
    pub name: String,
    pub field_type: FieldType,
    pub length_type: LengthType,
    pub sensitivity: SensitivityType,
    pub length: usize,
}

impl FieldSpec {
    pub fn min_value_size(&self) -> usize {
        // TODO: support codecs for LL
        match self.length_type {
            LengthType::Fixed => self.length,
            LengthType::LVar => 1,
            LengthType::LLVar => 1,
            LengthType::LLLVar => 1,
            LengthType::LLLLVar => 1,
            _ => 0,
        }
    }

    pub fn max_value_size(&self) -> usize {
        // TODO: support codecs for LL
        match self.length_type {
            LengthType::Fixed => self.length,
            LengthType::LVar => min(self.length, 9),
            LengthType::LLVar => min(self.length, 99),
            LengthType::LLLVar => min(self.length, 999),
            LengthType::LLLLVar => min(self.length, 9999),
            _ => 0,
        }
    }

    fn parse_length_prefix(
        &self,
        codec: &Codec,
        cursor: &mut Bytes,
        mut len: usize,
    ) -> Result<usize, RS8583Error> {
        if len == 0 {
            return Ok(0);
        }
        if cursor.remaining() < len {
            return Err(RS8583Error::parse_error(format!(
                "Unable to read length prefix ({} chars needed, {} available)",
                len,
                cursor.remaining()
            )));
        }
        let mut sz: usize = 0;
        while len > 0 {
            let len_byte = cursor.get_u8();
            sz += codec.byte_to_length(len_byte)? * 10usize.pow(len as u32 - 1);
            len -= 1;
        }
        if sz > self.length {
            return Err(RS8583Error::parse_error(format!(
                "Variable length field over max length ({} > {})",
                sz, self.length
            )));
        }
        Ok(sz)
    }

    pub fn to_read(&self, codec: &Codec, cursor: &mut Bytes) -> Result<usize, RS8583Error> {
        match &self.length_type {
            LengthType::BitMap => Ok(0),
            LengthType::Fixed => Ok(self.length),
            n => self.parse_length_prefix(codec, cursor, codec.length_size_bytes(n.length_size())),
        }
    }

    pub fn serialize_field(&self, codec: &Codec, buf: &mut BytesMut, field: &Field) -> Result<(), RS8583Error> {
        match &self.length_type {
            LengthType::BitMap => Ok(()),
            LengthType::Fixed => {
                if self.length == field.len() {
                    buf.extend_from_slice(field.as_slice());
                    Ok(())
                } else {
                    Err(RS8583Error::parse_error("Invalid field length"))
                }
            }
            n => {
                // TODO: check max data_len
                codec.serialize_prefix(buf, n.length_size(), field.len())?;
                buf.extend_from_slice(field.as_slice());
                Ok(())
            }
        }
    }
}

#[derive(Default)]
pub struct MessageSpec {
    pub fields: Vec<Option<FieldSpec>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_to_read_fixed() {
        let codec = Codec::default();

        let fs = FieldSpec {
            name: String::from("TEST"),
            field_type: FieldType::ANS,
            length_type: LengthType::Fixed,
            sensitivity: SensitivityType::Normal,
            length: 8,
        };

        let mut bytes = Bytes::from("TEST1234");

        assert_eq!(fs.to_read(&codec, &mut bytes).unwrap(), 8);
    }

    #[test]
    fn fs_to_read_lvar() {
        let codec = Codec::default();

        let fs = FieldSpec {
            name: String::from("TEST"),
            field_type: FieldType::ANS,
            length_type: LengthType::LVar,
            sensitivity: SensitivityType::Normal,
            length: 8,
        };

        let mut bytes = Bytes::from("3ABC");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(3));

        let mut bytes = Bytes::from("0ABC");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(0));

        let mut bytes = Bytes::from("9ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Variable length field over max length (9 > 8)"),
            })
        );

        let mut bytes = Bytes::from("");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Unable to read length prefix (1 chars needed, 0 available)"),
            })
        );

        let mut bytes = Bytes::from("!ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x21"),
            })
        );

        let mut bytes = Bytes::from("ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x41"),
            })
        );
    }

    #[test]
    fn fs_to_read_llvar() {
        let codec = Codec::default();

        let fs = FieldSpec {
            name: String::from("TEST"),
            field_type: FieldType::ANS,
            length_type: LengthType::LLVar,
            sensitivity: SensitivityType::Normal,
            length: 12,
        };

        let mut bytes = Bytes::from("03ABC");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(3));

        let mut bytes = Bytes::from("11ABCABCABCAB");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(11));

        let mut bytes = Bytes::from("00ABC");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(0));

        let mut bytes = Bytes::from("13ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Variable length field over max length (13 > 12)"),
            })
        );

        let mut bytes = Bytes::from("");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Unable to read length prefix (2 chars needed, 0 available)"),
            })
        );

        let mut bytes = Bytes::from("1");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Unable to read length prefix (2 chars needed, 1 available)"),
            })
        );

        let mut bytes = Bytes::from("!1ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x21"),
            })
        );

        let mut bytes = Bytes::from("1!ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x21"),
            })
        );

        let mut bytes = Bytes::from("ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x41"),
            })
        );

        let mut bytes = Bytes::from("1ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x41"),
            })
        );
    }

    #[test]
    fn fs_to_read_lllvar() {
        let codec = Codec::default();

        let fs = FieldSpec {
            name: String::from("TEST"),
            field_type: FieldType::ANS,
            length_type: LengthType::LLLVar,
            sensitivity: SensitivityType::Normal,
            length: 110,
        };

        let mut bytes = Bytes::from("003ABC");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(3));

        let mut bytes = Bytes::from("011ABCABCABCAB");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(11));

        let mut bytes = Bytes::from("000ABC");
        assert_eq!(fs.to_read(&codec, &mut bytes), Ok(0));

        let mut bytes = Bytes::from("111ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Variable length field over max length (111 > 110)"),
            })
        );

        let mut bytes = Bytes::from("");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Unable to read length prefix (3 chars needed, 0 available)"),
            })
        );

        let mut bytes = Bytes::from("1");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Unable to read length prefix (3 chars needed, 1 available)"),
            })
        );

        let mut bytes = Bytes::from("11");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Unable to read length prefix (3 chars needed, 2 available)"),
            })
        );

        let mut bytes = Bytes::from("!10ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x21"),
            })
        );

        let mut bytes = Bytes::from("1!0ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x21"),
            })
        );

        let mut bytes = Bytes::from("11!ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x21"),
            })
        );

        let mut bytes = Bytes::from("ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x41"),
            })
        );

        let mut bytes = Bytes::from("1ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x41"),
            })
        );

        let mut bytes = Bytes::from("11ABC");
        assert_eq!(
            fs.to_read(&codec, &mut bytes),
            Err(RS8583Error::ParseError {
                error: String::from("Length byte out of range: 0x41"),
            })
        );
    }
}
