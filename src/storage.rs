use std::{
    fs::{File, OpenOptions},
    path::{Path},
    io::{Read, Write, Seek, SeekFrom, Error, ErrorKind}
};

use mmap_safe::{MappedFile};

pub trait StorageBackend {
    fn init(&mut self, name: String);

    fn get_name(&self) -> String;

    fn get_volume_size(&self) -> u64;

    fn read(&self, offset: u64, length: usize) -> Vec<u8>;

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<(), Error>;

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error>;

    fn close(&mut self);
}

// Driver: MmapBackend

pub struct MmapBackend {
    name: String,
    volume_size: u64,
    pointer: Option<MappedFile>
}

impl MmapBackend {
    pub fn new(name: String) -> MmapBackend {
        let mut mmap = MmapBackend {
            name: name.clone(),
            volume_size: 0_u64,
            pointer: None
        };
        mmap.init(name.clone());
        mmap
    }
}

impl<'a> StorageBackend for MmapBackend {
    fn init(&mut self, name: String) {
        if self.pointer.is_some() {
            return ()
        }
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(name.to_lowercase().clone())
            .expect("Unable to open file");
        let volume_size = f.metadata().unwrap().len();
        println!("Volume Size of export {} is: <{}>", name.clone().to_lowercase(), volume_size);
        let mapped_file = MappedFile::new(f).expect("Something went wrong");
        self.pointer = Some(mapped_file);
        self.volume_size = volume_size;
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_volume_size(&self) -> u64 {
        self.volume_size
    }

    fn read(&self, offset: u64, length: usize) -> Vec<u8> {
        let mut buffer = vec![0_u8; length as usize];
        buffer.copy_from_slice(&self.pointer.as_ref().unwrap().map(offset, length).unwrap());
        buffer
    }

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<(), Error> {
        let pointer = self.pointer.take();
        let mut mut_pointer = pointer
            .unwrap()
            .into_mut_mapping(offset, length)
            .map_err(|(e, _)| e)
            .unwrap();
        mut_pointer.copy_from_slice(&data);
        self.pointer = Some(mut_pointer.unmap());
        Ok(())
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error> {
        let pointer = self.pointer.take();
        let mut_pointer = pointer
            .unwrap()
            .into_mut_mapping(offset, length)
            .map_err(|(e, _)| e)
            .unwrap();
        /*
        let pointer = BorrowMut::<MappedFile>::borrow_mut(&mut self.pointer);
        */
        mut_pointer.flush();
        self.pointer = Some(mut_pointer.unmap());
        Ok(())
    }

    fn close(&mut self) {
        let pointer = self.pointer.as_ref().unwrap();
        drop(pointer);
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
        ShardedFile {
            name: name,
            volume_size: 0_u64,
            shard_size: default_shard_size,
            storage_path: path
        }
    }

    pub fn shard_index(&self, offset: u64) -> usize {
        (offset / &self.shard_size) as usize
    }
}

impl StorageBackend for ShardedFile {
    fn init(&mut self, name: String) {
        let path = Path::new(&self.storage_path).join(name.clone());
        if !path.is_dir() | !path.exists() {
            eprintln!("No directory found: '{}'", path.display());
        }
        let volume_size = path.metadata().unwrap().len();
        self.volume_size = volume_size;
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_volume_size(&self) -> u64 {
        self.volume_size
    }

    fn read(&self, offset: u64, length: usize) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        let start = self.shard_index(offset);
        let end = self.shard_index(offset + length as u64);
        for i in start..=end {
            let path = Path::new(&self.storage_path)
                .join(self.name.clone())
                .join("-")
                .join(i.to_string());
            let mut file = OpenOptions::new()
                .read(true)
                .open(path)
                .unwrap();
            let file_not_exists = !file.metadata().unwrap().is_file();
            if i == start {
                if file_not_exists {
                    let read_size = (self.shard_size - offset % self.shard_size) as usize;
                    buffer.extend_from_slice(&vec![0_u8; read_size]);
                    continue;
                } else {
                    file.seek(SeekFrom::Start(offset % self.shard_size));
                }
            } else if i == end {
                let read_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                if file_not_exists {
                    buffer.extend_from_slice(&vec![0_u8; read_size]);
                    continue;
                } else {
                    let mut fill_buffer = vec![0_u8; read_size];
                    file.read_exact(&mut fill_buffer);
                    buffer.extend_from_slice(&fill_buffer);
                    //file.read_exact(&mut buffer);
                    continue;
                }
            }
            if file_not_exists {
                buffer.extend_from_slice(&vec![0_u8; self.shard_size as usize]);
            } else {
                file.read_to_end(&mut buffer).expect(&format!("couldn't read from file: {:?}-{}", self.name.clone(), i.to_string()));
            }
        }
        buffer
    }

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<(), Error> {
        let start = self.shard_index(offset);
        let end = self.shard_index(offset + length as u64);
        for i in start..=end {
            let mut file = OpenOptions::new()
                .write(true)
                .open(Path::new(&self.storage_path)
                    .join(self.name.clone())
                    .join("-")
                    .join(i.to_string())
                ).unwrap();
            let file_not_exists = !file.metadata().unwrap().is_file();
            if file_not_exists {
                file = File::create(Path::new(&self.storage_path)
                    .join(self.name.clone())
                    .join("-")
                    .join(i.to_string())
                    ).unwrap();
            }
            let range_start = (offset % self.shard_size + (i as u64) * self.shard_size) as usize;
            let range_end = (offset % self.shard_size + (i as u64 + 1) * self.shard_size) as usize;

            if i == start {
                let read_size = (self.shard_size - offset % self.shard_size) as usize;
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
                    continue;
                } else {
                    file.write_all(&data[range_start..(range_start + read_size)])?;
                    //file.sync_all()?
                    continue;
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
        Ok(())
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), Error> {
        let start = self.shard_index(offset);
        let end = self.shard_index(offset + length as u64);
        let mut result = Ok(());
        for i in start..=end {
            let path = Path::new(&self.storage_path)
                .join(self.name.clone())
                .join("-")
                .join(i.to_string());
            let file = OpenOptions::new()
                .open(path)
                .unwrap();
            result = file.sync_all();
            if result.is_err() {
                break;
            }
        }
        result
    }

    fn close(&mut self) {
        println!("Closed");
    }
}
