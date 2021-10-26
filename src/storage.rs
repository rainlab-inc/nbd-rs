use std::{
    fs::{OpenOptions}
};

use mmap_safe::{MappedFile};

pub trait StorageBackend {
    fn init(&mut self, name: String);

    fn get_name(&self) -> String;

    fn get_volume_size(&self) -> u64;

    fn read(&self, offset: u64, length: usize) -> Vec<u8>;

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<(), String>;

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), String>;

    fn close(&mut self);
}

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

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<(), String> {
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

    fn flush(&mut self, offset: u64, length: usize) -> Result<(), String> {
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
