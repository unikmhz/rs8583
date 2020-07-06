use bitmaps::Bitmap;
use bytes::{Buf, Bytes};
use typenum::{U192, U64};

use crate::error::RS8583Error;
use crate::field::Field;
use crate::spec::MessageSpec;

pub struct MTI([u8; 4]);

impl Default for MTI {
    fn default() -> Self {
        MTI(b"0000".to_owned())
    }
}

impl MTI {
    fn from_cursor(cursor: &mut Bytes) -> MTI {
        let mut mti = MTI::default();

        cursor.copy_to_slice(&mut mti.0);
        mti
    }

    pub fn version_byte(&self) -> u8 {
        self.0[0]
    }

    pub fn class_byte(&self) -> u8 {
        self.0[1]
    }

    pub fn function_byte(&self) -> u8 {
        self.0[2]
    }

    pub fn origin_byte(&self) -> u8 {
        self.0[3]
    }

    pub fn is_version_1987(&self) -> bool {
        self.version_byte() == b'0'
    }

    pub fn is_version_1993(&self) -> bool {
        self.version_byte() == b'1'
    }

    pub fn is_version_2003(&self) -> bool {
        self.version_byte() == b'2'
    }

    pub fn is_version_national(&self) -> bool {
        self.version_byte() == b'8'
    }

    pub fn is_version_private(&self) -> bool {
        self.version_byte() == b'9'
    }

    pub fn is_authorization(&self) -> bool {
        self.class_byte() == b'1'
    }

    pub fn is_financial(&self) -> bool {
        self.class_byte() == b'2'
    }

    pub fn is_file_action(&self) -> bool {
        self.class_byte() == b'3'
    }

    pub fn is_reversal(&self) -> bool {
        self.class_byte() == b'4'
    }

    pub fn is_reconciliation(&self) -> bool {
        self.class_byte() == b'5'
    }

    pub fn is_administrative(&self) -> bool {
        self.class_byte() == b'6'
    }

    pub fn is_fee_collection(&self) -> bool {
        self.class_byte() == b'7'
    }

    pub fn is_management(&self) -> bool {
        self.class_byte() == b'8'
    }

    pub fn is_reserved_class(&self) -> bool {
        self.class_byte() == b'9'
    }

    pub fn is_request(&self) -> bool {
        self.function_byte() == b'0'
    }

    pub fn is_request_response(&self) -> bool {
        self.function_byte() == b'1'
    }

    pub fn is_advice(&self) -> bool {
        self.function_byte() == b'2'
    }

    pub fn is_advice_response(&self) -> bool {
        self.function_byte() == b'3'
    }

    pub fn is_notification(&self) -> bool {
        self.function_byte() == b'4'
    }

    pub fn is_notification_ack(&self) -> bool {
        self.function_byte() == b'5'
    }

    pub fn is_instruction(&self) -> bool {
        self.function_byte() == b'6'
    }

    pub fn is_instruction_ack(&self) -> bool {
        self.function_byte() == b'7'
    }

    pub fn is_positive_ack(&self) -> bool {
        self.function_byte() == b'8'
    }

    pub fn is_negative_ack(&self) -> bool {
        self.function_byte() == b'9'
    }

    pub fn is_from_acquirer(&self) -> bool {
        match self.origin_byte() {
            b'0' | b'1' => true,
            _ => false,
        }
    }

    pub fn is_from_issuer(&self) -> bool {
        match self.origin_byte() {
            b'2' | b'3' => true,
            _ => false,
        }
    }

    pub fn is_from_other(&self) -> bool {
        match self.origin_byte() {
            b'4' | b'5' => true,
            _ => false,
        }
    }

    pub fn is_repeat(&self) -> bool {
        match self.origin_byte() {
            b'1' | b'3' | b'5' => true,
            _ => false,
        }
    }
}

// TODO: buffer size checks, everywhere

pub struct Message<'spec> {
    mti: MTI,
    bitmap: Bitmap<U192>,
    spec: &'spec MessageSpec,
    fields: Vec<Option<Field>>,
}

impl<'spec> Message<'spec> {
    pub fn from_bytes(spec: &'spec MessageSpec, raw: &mut Bytes) -> Result<Self, RS8583Error> {
        let mti = Self::parse_mti(raw);
        let bitmap = Self::parse_bitmap(raw);
        let fields = Self::parse_fields(spec, &bitmap, raw)?;
        Ok(Message {
            mti,
            bitmap,
            spec,
            fields,
        })
    }

    fn parse_mti(cursor: &mut Bytes) -> MTI {
        MTI::from_cursor(cursor)
    }

    fn parse_bitmap(cursor: &mut Bytes) -> Bitmap<U192> {
        let mut bm: Bitmap<U192> = Bitmap::new();
        let mut num_chunks = 0;

        // TODO: efficient copy
        loop {
            let chunk: Bitmap<U64> = Bitmap::from_value(cursor.get_u64_le());

            for bit in chunk.into_iter() {
                bm.set((num_chunks * 64) + bit, true);
            }
            num_chunks += 1;

            if !chunk.get(0) {
                break;
            }
        }
        bm
    }

    fn parse_fields(
        spec: &'spec MessageSpec,
        bitmap: &Bitmap<U192>,
        cursor: &mut Bytes,
    ) -> Result<Vec<Option<Field>>, RS8583Error> {
        let mut fields = vec![None; 192];

        for idx in bitmap {
            let field_spec = spec.fields.get(idx).unwrap();
            if field_spec.is_none() {
                // WARN
                continue;
            }
            let field_spec = field_spec.as_ref().unwrap();
            let to_read = field_spec.to_read(cursor)?;
            fields[idx] = Some(Field {
                data: Vec::from(&cursor[..to_read]),
                length: to_read,
            });
            cursor.advance(to_read);
        }

        Ok(fields)
    }

    pub fn mti(&self) -> &MTI {
        return &self.mti;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;

    fn test_spec() -> MessageSpec {
        MessageSpec {
            fields: vec![
                None,
                Some(FieldSpec {
                    name: String::from("TEST FIELD 2"),
                    field_type: FieldType::ANS,
                    length_type: LengthType::Fixed,
                    sensitivity: SensitivityType::Normal,
                    length: 12,
                }),
                Some(FieldSpec {
                    name: String::from("TEST FIELD 3"),
                    field_type: FieldType::ANS,
                    length_type: LengthType::Fixed,
                    sensitivity: SensitivityType::Normal,
                    length: 4,
                }),
                None,
                Some(FieldSpec {
                    name: String::from("TEST FIELD 5"),
                    field_type: FieldType::ANS,
                    length_type: LengthType::Fixed,
                    sensitivity: SensitivityType::Normal,
                    length: 2,
                }),
                None,
                Some(FieldSpec {
                    name: String::from("TEST FIELD 6"),
                    field_type: FieldType::ANS,
                    length_type: LengthType::LLVar,
                    sensitivity: SensitivityType::Normal,
                    length: 20,
                }),
            ],
        }
    }

    #[test]
    fn message_from_bytes() -> Result<(), RS8583Error> {
        let spec = test_spec();
        let raw = b"0120\x56\x00\x00\x00\x00\x00\x00\x00111122223333ABCDXY05LLVAR".to_vec();
        let mut bytes = Bytes::from(raw);
        let msg = Message::from_bytes(&spec, &mut bytes)?;

        let mti = msg.mti();
        assert_eq!(&mti.0, b"0120");

        assert!(mti.is_version_1987());
        assert!(mti.is_authorization());
        assert!(mti.is_advice());
        assert!(mti.is_from_acquirer());
        assert!(!mti.is_repeat());

        assert_eq!(msg.bitmap.get(0), false);
        assert_eq!(msg.bitmap.get(1), true);
        assert_eq!(msg.bitmap.get(2), true);
        assert_eq!(msg.bitmap.get(3), false);
        assert_eq!(msg.bitmap.get(4), true);
        assert_eq!(msg.bitmap.get(5), false);
        assert_eq!(msg.bitmap.get(6), true);
        assert_eq!(msg.bitmap.get(7), false);
        assert_eq!(msg.bitmap.get(63), false);

        assert!(msg.fields[0].is_none());
        assert!(msg.fields[1].is_some());

        let fld = msg.fields[1].as_ref().unwrap();
        assert_eq!(fld.data, b"111122223333");
        assert_eq!(fld.length, 12);

        let fld = msg.fields[2].as_ref().unwrap();
        assert_eq!(fld.data, b"ABCD");
        assert_eq!(fld.length, 4);

        let fld = msg.fields[3].as_ref();
        assert!(fld.is_none());

        let fld = msg.fields[4].as_ref().unwrap();
        assert_eq!(fld.data, b"XY");
        assert_eq!(fld.length, 2);

        let fld = msg.fields[5].as_ref();
        assert!(fld.is_none());

        let fld = msg.fields[6].as_ref().unwrap();
        assert_eq!(fld.data, b"LLVAR");
        assert_eq!(fld.length, 5);

        Ok(())
    }
}
