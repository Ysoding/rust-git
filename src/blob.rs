use std::any::Any;

use crate::Object;

pub struct Blob {
    pub blobdata: Vec<u8>,
}

impl Blob {
    pub fn new(data: &[u8]) -> Self {
        Self {
            blobdata: data.to_vec(),
        }
    }

    pub fn deserialize(data: &[u8]) -> Self {
        Self::new(data)
    }
}

impl Object for Blob {
    fn fmt(&self) -> &'static [u8] {
        b"blob"
    }

    fn serialize(&self) -> Vec<u8> {
        self.blobdata.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
