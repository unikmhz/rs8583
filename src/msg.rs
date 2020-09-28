use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::bitmap::BitMap;
use crate::codec::Codec;
use crate::error::RS8583Error;
use crate::field::Field;
use crate::spec::MessageSpec;

pub struct MTI([u8; 4]);

impl Default for MTI {
    fn default() -> Self {
        MTI([0x30, 0x30, 0x30, 0x30])
    }
}

impl MTI {
    fn from_cursor(cursor: &mut Bytes) -> Result<MTI, RS8583Error> {
        if cursor.remaining() < 4 {
            return Err(RS8583Error::parse_error("Truncated MTI"));
        }
        let mut mti = MTI::default();
        cursor.copy_to_slice(&mut mti.0);
        Ok(mti)
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
    bitmap: BitMap,
    spec: &'spec MessageSpec,
    fields: Vec<Option<Field>>,
}

impl<'spec> Message<'spec> {
    pub fn from_bytes(
        spec: &'spec MessageSpec,
        codec: &Codec,
        mut data: Bytes,
    ) -> Result<Self, RS8583Error> {
        let mti = MTI::from_cursor(&mut data)?;
        let bitmap = BitMap::from_cursor(&mut data)?;
        let fields = Self::parse_fields(spec, codec, &bitmap, &mut data)?;
        Ok(Message {
            mti,
            bitmap,
            spec,
            fields,
        })
    }

    fn parse_fields(
        spec: &'spec MessageSpec,
        codec: &Codec,
        bitmap: &BitMap,
        cursor: &mut Bytes,
    ) -> Result<Vec<Option<Field>>, RS8583Error> {
        let mut fields = vec![None; 128];

        for idx in bitmap.iter_set() {
            let field_spec = spec.fields.get(idx).unwrap();
            if field_spec.is_none() {
                // WARN
                continue;
            }
            let field_spec = field_spec.as_ref().unwrap();
            let to_read = field_spec.to_read(codec, cursor)?;
            if cursor.remaining() < to_read {
                // TODO: better error
                return Err(RS8583Error::parse_error("Truncated field"));
            }
            fields[idx] = Some(Field::from_bytes(cursor.slice(..to_read)));
            cursor.advance(to_read);
        }

        Ok(fields)
    }

    pub fn mti(&self) -> &MTI {
        &self.mti
    }

    pub fn field(&self, id: usize) -> Option<&Field> {
        if id >= self.fields.len() {
            None
        } else {
            self.fields[id].as_ref()
        }
    }

    pub fn set_field<T>(&mut self, idx: usize, value: T)
    where
        T: Into<Bytes>,
    {
        // TODO: check max idx
        // TODO: check value length (and possibly format)
        self.fields[idx] = Some(Field::from_bytes(value.into()));
        self.bitmap.set(idx);
    }

    pub fn clear_field(&mut self, idx: usize) {
        self.fields[idx] = None;
        self.bitmap.clear(idx);
    }

    pub fn serialize(&self, codec: &Codec) -> Result<BytesMut, RS8583Error> {
        // TODO: compute capacity
        let mut buf = BytesMut::with_capacity(32);

        // MTI
        buf.put(self.mti.0.as_ref());
        // BITMAP
        self.bitmap.serialize(&mut buf);
        // FIELDS
        for idx in self.bitmap.iter_set() {
            if let Some(field) = self.field(idx) {
                let field_spec = self.spec.fields.get(idx).unwrap();
                if field_spec.is_none() {
                    // WARN
                    continue;
                }
                let field_spec = field_spec.as_ref().unwrap();
                field_spec.serialize_field(codec, &mut buf, field)?;
            }
        }

        Ok(buf)
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
                Some(FieldSpec {
                    name: String::from("TEST FIELD 7"),
                    field_type: FieldType::B,
                    length_type: LengthType::Fixed,
                    sensitivity: SensitivityType::Normal,
                    length: 4,
                }),
            ],
        }
    }

    #[test]
    fn message_from_bytes() -> Result<(), RS8583Error> {
        let codec = Codec::default();
        let spec = test_spec();
        let raw = b"0120\x56\x00\x00\x00\x00\x00\x00\x00111122223333ABCDXY05LLVAR".to_vec();
        let orig_raw = raw.clone();
        let mut msg = Message::from_bytes(&spec, &codec, Bytes::from(raw))?;

        let mti = msg.mti();
        assert_eq!(&mti.0, b"0120");

        assert!(mti.is_version_1987());
        assert!(mti.is_authorization());
        assert!(mti.is_advice());
        assert!(mti.is_from_acquirer());
        assert!(!mti.is_repeat());

        assert_eq!(msg.bitmap.test(0), false);
        assert_eq!(msg.bitmap.test(1), true);
        assert_eq!(msg.bitmap.test(2), true);
        assert_eq!(msg.bitmap.test(3), false);
        assert_eq!(msg.bitmap.test(4), true);
        assert_eq!(msg.bitmap.test(5), false);
        assert_eq!(msg.bitmap.test(6), true);
        assert_eq!(msg.bitmap.test(7), false);
        assert_eq!(msg.bitmap.test(63), false);

        assert!(msg.fields[0].is_none());
        assert!(msg.fields[1].is_some());

        let fld = msg.field(1).unwrap();
        assert_eq!(fld.as_slice(), b"111122223333");
        assert_eq!(fld.len(), 12);

        let fld = msg.field(2).unwrap();
        assert_eq!(fld.as_slice(), b"ABCD");
        assert_eq!(fld.len(), 4);

        let fld = msg.field(3);
        assert!(fld.is_none());

        let fld = msg.field(4).unwrap();
        assert_eq!(fld.as_slice(), b"XY");
        assert_eq!(fld.len(), 2);

        let fld = msg.field(5);
        assert!(fld.is_none());

        let fld = msg.field(6).unwrap();
        assert_eq!(fld.as_slice(), b"LLVAR");
        assert_eq!(fld.len(), 5);

        let fld = msg.field(7);
        assert!(fld.is_none());

        let serialized = msg.serialize(&codec).unwrap();
        assert_eq!(serialized.as_ref(), &orig_raw[..]);
        assert_eq!(serialized.as_ref(), &orig_raw[..]);

        msg.set_field(7, "1234");

        let fld = msg.field(7).unwrap();
        assert_eq!(fld.as_slice(), b"1234");
        assert_eq!(fld.len(), 4);
        assert_eq!(msg.bitmap.test(7), true);

        let serialized = msg.serialize(&codec).unwrap();
        assert_eq!(
            serialized,
            Bytes::from(
                b"0120\xd6\x00\x00\x00\x00\x00\x00\x00111122223333ABCDXY05LLVAR1234".to_vec()
            )
        );

        Ok(())
    }
}
