use bytes::Bytes;

#[derive(Clone, Debug)]
pub struct Field {
    data: Bytes,
}

impl Field {
    pub fn from_bytes(data: Bytes) -> Self {
        Field { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.data.as_ref()
    }
}
