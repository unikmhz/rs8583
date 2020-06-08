use bytes::{Buf, Bytes};

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
            _ => 0
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
    pub fn to_read(&self, cursor: &Bytes) -> usize {
        match self.length_type {
            LengthType::BitMap => 0,
            LengthType::Fixed => self.length,
            LengthType::LVar => 1, // FIXME
            LengthType::LLVar => 2, // FIXME
            LengthType::LLLVar => 3, // FIXME
            LengthType::LLLLVar => 4, // FIXME
        }
    }
}

#[derive(Default)]
pub struct MessageSpec {
    pub fields: Vec<Option<FieldSpec>>,
}
