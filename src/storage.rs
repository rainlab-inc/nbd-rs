use std::{
    fs::{OpenOptions},
    path::{Path},
    io::{Read, Write, Seek, SeekFrom, Error, ErrorKind},
};

extern crate libc;

use crate::{
    object::{ObjectStorage, storage_with_config},
};

pub trait StorageBackend {
    fn init(&mut self, name: String);

    fn get_name(&self) -> String;

    fn get_volume_size(&self) -> u64;

    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, Error>;

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error>;

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error>;

    fn close(&mut self);
}

// Driver: RawImage

pub struct RawImage {
    name: String,
    volume_size: u64,
    objectStorage: Box<dyn ObjectStorage>,
}

impl RawImage {
    pub fn new(name: String, config: String) -> RawImage {
        let mut selfref = RawImage {
            name: name.clone(),
            volume_size: 0_u64,
            objectStorage: Box::new(object::storage_with_config(config)),
        };
        selfref.init(name.clone());
        selfref
    }
}

impl<'a> StorageBackend for RawImage {
    fn init(&mut self, name: String) {
        if self.pointer.is_some() {
            return ()
        }
        // TODO: Init Object Storage
        self.objectStorage
            .startOperationsOnObject(name.clone());
            // .expect("Unable to open object");

        self.volume_size = self.objectStorage.get_size(name.clone()).unwrap_or(0);
        self.name = name.clone();
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_volume_size(&self) -> u64 {
        self.volume_size
    }

    fn read(&self, offset: u64, length: usize) -> Result<&[u8], Error> {
        self.objectStorage
            .readPartial(self.name.clone(), offset, length)
    }

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        self.objectStorage
            .writePartial(self.name.clone(), offset, length, data)
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error> {
        self.objectStorage
            .persistObject(self.name.clone())
    }

    fn close(&mut self) {
        self.objectStorage
            .endOperationsOnObject(self.name.clone())
    }
}

// Driver: ShardedFile

pub struct ShardedFile {
    name: String,
    volume_size: u64,
    shard_size: u64,
    storage_path: String
}

impl ShardedFile {
    pub fn new(name: String, path: String) -> ShardedFile {
        let default_shard_size: u64 = 4 * 1024 * 1024;
        let mut sharded_file = ShardedFile {
            name: name.clone(),
            volume_size: 0_u64,
            shard_size: default_shard_size,
            storage_path: path
        };
        sharded_file.init(name);
        sharded_file
    }

    pub fn shard_index(&self, offset: u64) -> usize {
        (offset / &self.shard_size) as usize
    }

    pub fn size_of_volume(&self, dir: &Path) -> u64 {
        let path = dir.join("size");
        if !path.is_file() | !path.exists() {
            eprintln!("No metadata file found: '{}'", path.display());
        }
        let mut string = std::fs::read_to_string(path).unwrap();
        string.retain(|c| !c.is_whitespace());
        let volume_size: u64 = string.parse().unwrap();
        volume_size
    }
}

impl StorageBackend for ShardedFile {
    fn init(&mut self, name: String) {
        let path = Path::new(&self.storage_path).join(self.name.clone());
        if !path.is_dir() | !path.exists() {
            eprintln!("No directory found: '{}'", path.display());
        }
        self.volume_size = self.size_of_volume(&path);
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_volume_size(&self) -> u64 {
        self.volume_size
    }

    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = Vec::new();
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };
        println!("Start: {}, End: {}", start, end);
        for i in start..=end {
            println!("Iteration: {}", i);
            let path = Path::new(&self.storage_path)
                .join(self.name.clone())
                .join(format!("{}-{}", self.name.clone(), i.to_string()));
            if path.is_file() {
                let mut file = OpenOptions::new()
                    .read(true)
                    .open(path)
                    .unwrap();
                if i == start {
                    let read_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                    file.seek(SeekFrom::Start(offset % self.shard_size));
                    let mut buf = vec![0_u8; read_size];
                    file.read_exact(&mut buf).expect("read failed");
                    buffer.extend_from_slice(&buf);
                    continue;
                }
                if i == end {
                    let read_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                    let mut buf = vec![0_u8; read_size];
                    file.read_exact(&mut buf).expect("read failed");
                    buffer.extend_from_slice(&buf);
                    break;
                }
                file.read_to_end(&mut buffer).expect(&format!("couldn't read from file: {:?}-{}", self.name.clone(), i.to_string()));
            } else {
                if i == start {
                    let read_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                    buffer.extend_from_slice(&vec![0_u8; read_size]);
                    continue;
                }
                if i == end {
                    let read_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                    buffer.extend_from_slice(&vec![0_u8; read_size]);
                    break;
                }
                buffer.extend_from_slice(&vec![0_u8; self.shard_size as usize]);
            }
        }
        Ok(buffer)
    }

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };
        println!("Start: {}, End: {}", start, end);
        for i in start..=end {
            println!("Iteration: {}", i);
            let path = Path::new(&self.storage_path)
                .join(self.name.clone())
                .join(format!("{}-{}", self.name.clone(), i.to_string()));
            let file_not_exists = !&path.is_file();
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(path)
                .unwrap();
            let range_start = (offset % self.shard_size + (i as u64) * self.shard_size) as usize;
            let range_end = (offset % self.shard_size + (i as u64 + 1) * self.shard_size) as usize;

            if i == start {
                let read_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                if file_not_exists {
                    let zeroes: Vec<u8> = vec![0_u8; self.shard_size as usize - read_size];
                    let mut buffer: Vec<u8> = Vec::new();
                    buffer.extend_from_slice(&zeroes);
                    buffer.extend_from_slice(&data[0..read_size]);
                    file.write_all(&buffer)?;
                    //file.sync_all()?
                    continue;
                } else {
                    file.seek(SeekFrom::Start(offset % self.shard_size));
                    file.write_all(&data[0..read_size])?;
                    //file.sync_all()?;
                    continue;
                }
            } else if i == end {
                let read_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                if file_not_exists {
                    let zeroes: Vec<u8> = vec![0_u8; self.shard_size as usize - read_size];
                    let mut buffer: Vec<u8> = Vec::new();
                    buffer.extend_from_slice(&data[range_start..(range_start + read_size)]);
                    buffer.extend_from_slice(&zeroes);
                    file.write_all(&buffer)?;
                    //file.sync_all()?
                    break;
                } else {
                    file.write_all(&data[range_start..(range_start + read_size)])?;
                    //file.sync_all()?
                    break;
                }
            }
            let err = file.write(&data[range_start..range_end]);
            if err.is_err() {
                return Err(Error::new(ErrorKind::Other, format!("Error at file: '{:?}'. {:?}", self.name.clone(), err)))
            }
            /*
            err = file.sync_all();
            if err.is_err() {
                return Err(Error::new(ErrorKind::Other, format!("Error at file: '{:?}'. {:?}", self.name.clone(), err)))
            }
            */
        }
        Ok(length)
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error> {
        unsafe{ libc::sync(); }
        /*
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };
        let mut result = Ok(());
        for i in start..=end {
            let path = Path::new(&self.storage_path)
                .join(self.name.clone())
                .join(format!("{}-{}", self.name.clone(), i.to_string()));
            let file_not_exists = !&path.is_file();
            if file_not_exists {
                println!("File does not exist: '{:?}'", path.display());
                continue;
            }
            let file = OpenOptions::new()
                .write(true)
                .open(path)
                .unwrap();
            result = file.sync_all();
            if result.is_err() {
                break;
            }
        }
        result
        */
        Ok(())
    }

    fn close(&mut self) {
        println!("Closed");
    }
}
