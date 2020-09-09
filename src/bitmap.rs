use bitvec::prelude::*;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::mem::size_of;

use crate::error::RS8583Error;

type BV = BitVec<Lsb0, u64>;

pub struct BitMap {
    inner: BV,
}

impl BitMap {
    pub fn from_cursor(cursor: &mut Bytes) -> Result<Self, RS8583Error> {
        // TODO: optimize: provide default capacity != 128?
        let mut inner = BitVec::with_capacity(128);

        loop {
            if cursor.remaining() < size_of::<u64>() {
                return Err(RS8583Error::parse_error("Truncated bitmap"));
            }
            let mut chunk: BV = BitVec::from_element(cursor.get_u64_le());
            let more = chunk[0];

            inner.append(&mut chunk);
            if !more {
                break;
            }
        }

        Ok(BitMap {
            inner,
        })
    }

    pub fn serialize(&self, buf: &mut BytesMut) {
        for chunk in self.inner.as_slice() {
            buf.put_u64_le(*chunk);
        }
    }

    fn resize_for_idx(&mut self, idx: usize) {
        let new_size = idx + 1;
        let new_size = new_size + (64 - new_size % 64);
        self.inner.resize(new_size, false);
    }

    pub fn test(&self, idx: usize) -> bool {
        if self.inner.len() > idx {
            self.inner[idx]
        } else {
            false
        }
    }

    pub fn set(&mut self, idx: usize) {
        if self.inner.len() <= idx {
            self.resize_for_idx(idx);
        }
        self.inner.set(idx, true);
        if idx > 63 {
            self.inner.set(idx - 64 - idx % 64, true);
        }
    }

    pub fn clear(&mut self, idx: usize) {
        if self.inner.len() > idx && self.inner[idx] {
            self.inner.set(idx, false);
            // TODO: cleanup
        }
    }

    pub fn iter_set(&self) -> impl Iterator<Item = usize> + '_ {
        self.inner
            .iter()
            .enumerate()
            .filter_map(|(idx, value)| {
                if idx % 64 == 0 {
                    None
                } else if *value {
                    Some(idx)
                } else {
                    None
                }
            })
    }
}
