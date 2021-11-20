use std::{
    io::{Error},
};

use crate::{
    object::{ObjectStorage, object_storage_with_config},
    block::{BlockStorage},
};

// Driver: RawBlock

pub struct RawBlock {
    name: String,
    volume_size: u64,
    object_storage: Box<dyn ObjectStorage>,
}

impl RawBlock {
    pub fn new(name: String, config: String) -> RawBlock {
        let mut selfref = RawBlock {
            name: name.clone(),
            volume_size: 0_u64,
            object_storage: object_storage_with_config(config).unwrap(),
        };
        selfref.init();
        selfref
    }
}

impl BlockStorage for RawBlock {
    fn init(&mut self) {
        self.object_storage
            .start_operations_on_object(self.name.clone()).unwrap();

        self.volume_size = self.object_storage.get_size(self.name.clone()).unwrap_or(0);
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_volume_size(&self) -> u64 {
        self.volume_size
    }

    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        self.object_storage
            .partial_read(self.name.clone(), offset, length)
    }

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        self.object_storage
            .partial_write(self.name.clone(), offset, length, data)
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error> {
        self.object_storage
            .persist_object(self.name.clone())
    }

    fn close(&mut self) {
        self.object_storage
            .end_operations_on_object(self.name.clone())
            .expect("Could not close object properly");
    }
}
