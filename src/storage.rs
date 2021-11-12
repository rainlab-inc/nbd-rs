use std::{
    str,
    io::{Error},
};

use crate::{
    object::{ObjectStorage, storage_with_config},
};

pub trait StorageBackend {
    fn init(&mut self);
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
    object_storage: Box<dyn ObjectStorage>,
}

impl RawImage {
    pub fn new(name: String, config: String) -> RawImage {
        let mut selfref = RawImage {
            name: name.clone(),
            volume_size: 0_u64,
            object_storage: storage_with_config(config).unwrap(),
        };
        selfref.init();
        selfref
    }
}

impl<'a> StorageBackend for RawImage {
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

// Driver: ShardedFile

pub struct ShardedFile {
    name: String,
    volume_size: u64,
    shard_size: u64,
    object_storage: Box<dyn ObjectStorage>,
}

impl ShardedFile {
    pub fn new(name: String, config: String) -> ShardedFile {
        let default_shard_size: u64 = 4 * 1024 * 1024;
        let mut sharded_file = ShardedFile {
            name: name.clone(),
            volume_size: 0_u64,
            shard_size: default_shard_size,
            object_storage: storage_with_config(config).unwrap(),
        };
        sharded_file.init();
        sharded_file
    }

    pub fn shard_index(&self, offset: u64) -> usize {
        (offset / &self.shard_size) as usize
    }

    pub fn size_of_volume(&self) -> u64 {
        let shard_name = format!("{}-size", self.name.clone());
        let filedata = self.object_storage.read(shard_name).unwrap(); // TODO: Errors?
        let mut string = str::from_utf8(&filedata).unwrap().to_string();
        string.retain(|c| !c.is_whitespace());
        let volume_size: u64 = string.parse().unwrap();
        volume_size
    }
}

impl StorageBackend for ShardedFile {
    fn init(&mut self) {
        self.volume_size = self.size_of_volume();
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

        println!("(Read) Start: {}, End: {}", start, end);
        for i in start..=end {
            println!("(Read) Iteration: {}", i);
            let shard_name = format!("{}-{}", self.name.clone(), i.to_string());

            if self.object_storage.exists(shard_name.clone())? {
                if i == start {
                    let read_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                    let buf = self.object_storage
                        .partial_read(shard_name.clone(), offset % self.shard_size, read_size)?;
                    buffer.extend_from_slice(&buf);
                    continue;
                }
                if i == end {
                    let read_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                    let buf = self.object_storage
                        .partial_read(shard_name.clone(), 0, read_size)?;
                    buffer.extend_from_slice(&buf);
                    break;
                }
                let buf = self.object_storage
                    .read(shard_name.clone())?;
                buffer.extend_from_slice(&buf);
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
        println!("(Write) Start: {}, End: {}", start, end);
        for i in start..=end {
            println!("(Write) Iteration: {}", i);
            let shard_name = format!("{}-{}", self.name.clone(), i.to_string());

            let range_start = (offset % self.shard_size + (i as u64) * self.shard_size) as usize;
            let range_end = (offset % self.shard_size + (i as u64 + 1) * self.shard_size) as usize;

            if i == start {
                let write_len = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                if !self.object_storage.exists(shard_name.clone())? {
                    let zeroes: Vec<u8> = vec![0_u8; self.shard_size as usize - write_len];
                    let mut buffer: Vec<u8> = Vec::new();
                    buffer.extend_from_slice(&zeroes);
                    buffer.extend_from_slice(&data[0..write_len]);

                    self.object_storage.write(shard_name.clone(), &buffer)?;
                    continue;
                } else {
                    self.object_storage.partial_write(shard_name.clone(), offset % self.shard_size, write_len, &data[0..write_len])?;
                    continue;
                }
            } else if i == end {
                let write_len = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                if !self.object_storage.exists(shard_name.clone())? {
                    let zeroes: Vec<u8> = vec![0_u8; self.shard_size as usize - write_len];
                    let mut buffer: Vec<u8> = Vec::new();
                    buffer.extend_from_slice(&data[range_start..(range_start + write_len)]);
                    buffer.extend_from_slice(&zeroes);
                    self.object_storage.write(shard_name.clone(), &buffer)?;
                    break;
                } else {
                    self.object_storage.partial_write(shard_name.clone(), 0, write_len, &data[range_start..(range_start + write_len)])?;
                    break;
                }
            }

            self.object_storage.write(shard_name.clone(), &data[range_start..range_end])?;
        }
        Ok(length)
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error> {
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };
        println!("(Flush) Start: {}, End: {}", start, end);
        for i in start..=end {
            println!("(Flush) Iteration: {}", i);
            let shard_name = format!("{}-{}", self.name.clone(), i.to_string());
            self.object_storage.persist_object(shard_name.clone())?;
        }

        Ok(())
    }

    fn close(&mut self) {
        println!("Closed");
    }
}
