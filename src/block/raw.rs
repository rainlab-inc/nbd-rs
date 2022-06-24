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
    volume_size: u64,
    object_storage: Box<dyn ObjectStorage>,
    volume_initialized: bool,
    config: BlockStorageConfig,
}

impl RawBlock {
    pub fn new(config: BlockStorageConfig) -> RawBlock {
        let mut split: Vec<&str> = config.conn_str.split(":").collect();
        let driver_name = split.remove(0);
        let driver_config = split.join(":");

        let segments = Url::from_file_path(&driver_config).unwrap();
        let filename = segments.path_segments().unwrap().last().unwrap();
        let new_config = segments.as_str().strip_suffix(filename).unwrap();

        let selfref = RawBlock {
            export_name: config.export_name.clone(),
            name: String::from(filename),
            volume_size: 0_u64,
            object_storage: object_storage_with_config(String::from(new_config)).unwrap(),
            volume_initialized: false,
            config: config.clone(),
        };

        selfref
    }
}

impl BlockStorage for RawBlock {
    fn init(&mut self) {
        assert!(self.volume_initialized, "Should initialize the volume first");
        self.object_storage.start_operations_on_object(self.name.clone()).unwrap();
    }

    fn init_volume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.volume_initialized {
            return Err(Error::new(ErrorKind::Other, format!("Volume is already initialized.")).into());
        }


        let path = Path::new(self.name.as_str());
        /* Check file is already exist */
        if path.is_file() {
            let file = File::open(path)?;
            let size = file.metadata()?.len();

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

        

        let mut file = File::create(self.name.clone())?;
        let volume_size = self.config.export_size.unwrap() as u64;
        log::info!("Initializing volume: {} with size: {}", self.name, volume_size);

        file.seek(SeekFrom::Start(volume_size - 1))?;
        file.write_all(&[0_u8])?;

        
        log::info!("Volume size is written.");
        
        self.volume_size = volume_size;
        self.volume_initialized = true;
        self.init();
        Ok(())
    }
    
    fn init_volume_from_remote(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.volume_initialized {
            return Err(Error::new(ErrorKind::Other, format!("Volume is already initialized.")).into());
        }
        
        if self.config.export_name.is_some() && self.config.export_size.is_none() {
            let volume_size = std::fs::metadata(self.name.clone())?.len();
            log::info!("Volume size of the block stoage is {}", volume_size);
            self.volume_size = volume_size;
            self.volume_initialized = true;
            self.init();
            Ok(())
        } else {
            return Err(Error::new(ErrorKind::Other, format!("init_volume_from_remote() is failed.")).into());
        }
    }
    
    fn destroy_volume(&mut self) {
        self.object_storage.destroy();
        self.volume_initialized = false;
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
