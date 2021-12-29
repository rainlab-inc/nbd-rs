use std::{
    io::{Error}
};
use url::{Url};

use crate::{
    object::{ObjectStorage, object_storage_with_config},
    block::{BlockStorage},
};
use crate::util::Propagation;

// Driver: RawBlock

pub struct RawBlock {
    export_name: String,
    name: String,
    volume_size: u64,
    object_storage: Box<dyn ObjectStorage>,
}

impl RawBlock {
    pub fn new(name: String, config: String) -> RawBlock {
        let mut split: Vec<&str> = config.split(":").collect();
        let driver_name = split.remove(0);
        let driver_config = split.join(":");

        let segments = Url::from_file_path(&driver_config).unwrap();
        let filename = segments.path_segments().unwrap().last().unwrap();
        let new_config = segments.as_str().strip_suffix(filename).unwrap();

        let mut selfref = RawBlock {
            export_name: name.clone(),
            name: String::from(filename),
            volume_size: 0_u64,
            object_storage: object_storage_with_config(String::from(new_config)).unwrap(),
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

    fn supports_trim(&self) -> bool {
        false
    }

    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        self.object_storage
            .partial_read(self.name.clone(), offset, length)
    }

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error> {
        self.object_storage
            .partial_write(self.name.clone(), offset, length, data)
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<Propagation, Error> {
        self.object_storage
            .persist_object(self.name.clone())
    }

    fn close(&mut self) {
        self.object_storage
            .end_operations_on_object(self.name.clone())
            .expect("Could not close object properly");
        self.object_storage.close();
    }
}
