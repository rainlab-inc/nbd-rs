use std::{
    io::{Error, ErrorKind, SeekFrom, Seek, Write},
    fs::{File},
    path::{Path},
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
    path: String,
    volume_size: u64,
    object_storage: Box<dyn ObjectStorage>,
    config: BlockStorageConfig,
}

impl RawBlock {
    pub fn new(config: BlockStorageConfig) -> RawBlock {
        let segments = Url::parse(&config.conn_str).unwrap();
        let filename = segments.path_segments().unwrap().last().unwrap();
        let new_config = segments.as_str().strip_suffix(filename).unwrap();

        let object_storage = object_storage_with_config(String::from(new_config)).unwrap();
        if !object_storage.supports_random_write_access() {
            panic!("Object storage should support random write access for RawBlock.");
        }

        let mut selfref = RawBlock {
            export_name: config.export_name.clone(),
            name: String::from(filename),
            path: String::from(segments.path()),
            volume_size: 0_u64,
            object_storage,
            config: config.clone(),
        };

        selfref.init(config.init_volume).unwrap();
        selfref
    }
}

impl BlockStorage for RawBlock {
    fn init(&mut self, init_volume: bool) -> Result<(), Box<dyn std::error::Error>> {
        if init_volume {
            self.init_volume()?;
        } else {
            self.check_volume()?;
        }

        self.object_storage.start_operations_on_object(self.name.clone())?;
        Ok(())
    }

    fn init_volume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.object_storage.exists(self.name.clone())? {
            let size = self.object_storage.get_size(self.name.clone())?;
            if size == self.config.export_size.unwrap() as u64 {
                log::warn!("Block storage is already initialized with the same size: {}", size);
            } else {
                if !self.config.export_force {
                    return Err(Error::new(ErrorKind::Other, format!("Block storage is already initialized and the size is configured to be {}, add --force to override current configuration", size )).into());
                } else {
                    log::warn!("Block storage is already initialized with size: {}", size);
                }
            }
        }

        let volume_size = self.config.export_size.unwrap() as u64;
        self.object_storage.create_object(self.name.clone(), volume_size);
        log::info!("Volume size is written.");
        
        self.volume_size = volume_size;
        Ok(())
    }
    
    fn check_volume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let volume_size = std::fs::metadata(self.path.clone())?.len();
        log::info!("Volume size of the block storage is {}", volume_size);
        self.volume_size = volume_size;
        Ok(())
    }
    
    fn destroy_volume(&mut self) {
        std::fs::remove_file(self.path.clone()).unwrap();
        log::info!("The volume({}) is destroyed.", self.path);
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
