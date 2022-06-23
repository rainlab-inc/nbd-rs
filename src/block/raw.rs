use std::{
    io::{Error}
};
use url::{Url};

use crate::{
    object::{ObjectStorage, object_storage_with_config},
    block::{BlockStorage, BlockStorageConfig},
};
use crate::util::Propagation;

// Driver: RawBlock

pub struct RawBlock {
    export_name: Option<String>,
    name: String,
    volume_size: u64,
    object_storage: Box<dyn ObjectStorage>,
}

impl RawBlock {
    pub fn new(config: BlockStorageConfig) -> RawBlock {
        let mut split: Vec<&str> = config.conn_str.split(":").collect();
        let driver_name = split.remove(0);
        let driver_config = split.join(":");

        let segments = Url::from_file_path(&driver_config).unwrap();
        let filename = segments.path_segments().unwrap().last().unwrap();
        let new_config = segments.as_str().strip_suffix(filename).unwrap();

        let mut selfref = RawBlock {
            export_name: config.export_name.clone(),
            name: String::from(filename),
            volume_size: 0_u64,
            object_storage: object_storage_with_config(String::from(new_config)).unwrap(),
        };

        if config.export_name.is_none() && config.export_size.is_some() {
            // Initialize volume
            let volume_size = config.export_size.unwrap() as u64;
            log::info!("Volume size: {}", volume_size);
            selfref.volume_size = volume_size;

            /* Check initialized */
            let size = selfref.object_storage.read("size".to_string());
            if size.is_ok() {
                /* Already initialized */
                let size = String::from_utf8(selfref.object_storage.read("size".to_string()).unwrap()).unwrap();
                let size: u64 = size.parse().unwrap();
                if size == volume_size {
                    log::warn!("Block storage is already initialized with the same size: {}", size);
                } else {
                    if !config.export_force {
                        log::error!("Block storage is already initialized and the size is configured to be {}, add --force to override current configuration", size);
                        panic!();
                    } else {
                        log::warn!("Block storage is already initialized with size: {}", size);
                    }
                }
            }
            let size_str = volume_size.to_string();
            selfref.object_storage.write(String::from("size"), &size_str.as_bytes());
            log::info!("Volume size is written.");

        } else if config.export_name.is_some() && config.export_size.is_none() {
            let size = String::from_utf8(selfref.object_storage.read("size".to_string()).unwrap()).unwrap();
            let size: u64 = size.parse().unwrap();
            log::info!("Volume size of the block stoage is {}", size);
            selfref.volume_size = size;
        } else {
            panic!("Unreachable");
        }

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
        self.object_storage.supports_trim()
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

    fn trim(&mut self, offset: u64, length: usize) -> Result<Propagation, Error> {
        self.object_storage
            .trim_object(self.name.clone(), offset, length)
    }

    fn close(&mut self) {
        self.object_storage
            .end_operations_on_object(self.name.clone())
            .expect("Could not close object properly");
        self.object_storage.close();
    }
}
