extern crate typenum;
extern crate bitmaps;

use std::borrow::Cow;
use bitmaps::Bitmap;
use bytes::{Buf, Bytes};
use typenum::{U192, U64};

use crate::spec::MessageSpec;
use crate::field::Field;

struct MTI([u8; 4]);

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
}

// TODO: buffer size checks, everywhere

pub struct Message<'spec> {
    mti: MTI,
    bitmap: Bitmap<U192>,
    spec: &'spec MessageSpec,
    fields: Vec<Option<Field>>,
}

impl<'spec> Message<'spec> {
    pub fn from_bytes(spec: &'spec MessageSpec, raw: &mut Bytes) -> Self {
        let mti = Self::parse_mti(raw);
        let bitmap = Self::parse_bitmap(raw);
        let fields = Self::parse_fields(spec, &bitmap, raw);
        Message {
            mti,
            bitmap,
            spec,
            fields,
        }
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

    fn parse_fields(spec: &'spec MessageSpec, bitmap: &Bitmap<U192>, cursor: &mut Bytes) -> Vec<Option<Field>> {
        let mut fields = vec![None; 192];

        for idx in bitmap {
            let field_spec = spec.fields.get(idx).unwrap();
            if field_spec.is_none() {
                // WARN
                continue;
            }
            let field_spec = field_spec.as_ref().unwrap();
            let to_read = field_spec.to_read(cursor);
            fields[idx] = Some(Field {
                data: Vec::from(&cursor[..to_read]),
                length: to_read,
            });
            cursor.advance(to_read);
        }

        fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;

    #[test]
    fn message_from_bytes() {
        let spec = MessageSpec {
            fields: vec![None, Some(FieldSpec {
                name: String::from("TEST FIELD 2"),
                field_type: FieldType::ANS,
                length_type: LengthType::Fixed,
                sensitivity: SensitivityType::Normal,
                length: 12,
            }), Some(FieldSpec {
                name: String::from("TEST FIELD 3"),
                field_type: FieldType::ANS,
                length_type: LengthType::Fixed,
                sensitivity: SensitivityType::Normal,
                length: 4,
            }), None, Some(FieldSpec{
                name: String::from("TEST FIELD 5"),
                field_type: FieldType::ANS,
                length_type: LengthType::Fixed,
                sensitivity: SensitivityType::Normal,
                length: 2,
            })]
        };
        let raw = b"0120\x16\x00\x00\x00\x00\x00\x00\x00111122223333ABCDXY".to_vec();
        let mut bytes = Bytes::from(raw);
        let msg = Message::from_bytes(&spec, &mut bytes);

        assert_eq!(&msg.mti.0, b"0120");
        assert_eq!(msg.bitmap.get(0), false);
        assert_eq!(msg.bitmap.get(1), true);
        assert_eq!(msg.bitmap.get(2), true);
        assert_eq!(msg.bitmap.get(3), false);
        assert_eq!(msg.bitmap.get(4), true);
        assert_eq!(msg.bitmap.get(5), false);
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
    }
}
