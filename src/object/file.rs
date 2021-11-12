use std::{
    fs::{File, OpenOptions},
    io::{Read, Write, Error, ErrorKind},
    rc::{Rc},
    collections::{HashMap}
};

use mmap_safe::{MappedFile};

use crate::object::{
    ObjectStorage,
    SimpleObjectStorage,
    PartialAccessObjectStorage,
    StreamingObjectStorage,
    StreamingPartialAccessObjectStorage,
};

pub struct FileBackend {
    folder_path: String,
    open_files: HashMap<String, Box<MappedFile>>,
}

impl Default for FileBackend {
    fn default() -> FileBackend {
        FileBackend {
            folder_path: String::from(""),
            open_files: HashMap::<String, Box<MappedFile>>::new()
        }
    }
}

impl FileBackend {
    fn open_file(&self, objectName: String, create: bool) -> Result<File, Error> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(create)
            .open(objectName.clone())
    }

    fn mmap_file(&self, objectName: String) -> Result<&Box<MappedFile>, Error> {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(objectName.clone())
            .expect("Unable to open file");

        let mapped_file = match self.open_files.get_key_value(&objectName.clone()) {
            Some(m) => m.1,
            None => {
                let mapped = MappedFile::new(f);
                self.open_files.insert(objectName.clone(), Box::new(mapped.unwrap()));
                &Box::new(mapped.unwrap())
            }
        };
        Ok(mapped_file)
    }

    fn get_file(&self, objectName: String) -> Result<&Box<MappedFile>, Error> {
        // TODO: Check if self.openFiles already has the file, return that
        //let file = self.open_file(objectName, false);
        self.mmap_file(objectName)
    }
}

impl<'a> SimpleObjectStorage for FileBackend {
    fn init(&mut self, connStr: String) {
        self.folder_path = connStr.clone()
    }

    fn exists(&self, objectName: String) -> Result<bool, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn read(&self, objectName: String) -> Result<Vec<u8>, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn write(&self, objectName: String, data: &[u8]) -> Result<(), Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn delete(&self, objectName: String) -> Result<(), Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    fn get_size (&self, objectName: String) -> Result<u64, Error> {
        let length_data = std::fs::metadata(objectName.clone());
        if length_data.is_ok() {
            Ok(length_data.unwrap().len())
        } else {
            Err(Error::new(ErrorKind::Other, format!("Error on getting size of: <{}>", objectName)))
        }
    }

    fn startOperationsOnObject (&self, objectName: String) -> Result<(), Error> {
        // TODO: Check if self.openFiles already has same file, use Rc.increment_strong_count in that case

        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(objectName.clone())
            .expect("Unable to open file");

        // TODO: Mmap? MappedFile::new(f).expect("Something went wrong");
        self.open_files.insert(objectName.clone(), Box::new(MappedFile::new(f).unwrap()));
        Ok(())
    }

    fn endOperationsOnObject(&self, objectName: String) -> Result<(), Error> {
        // TODO: code below is stupid here. just remove file from this.openFiles
        let file = self.get_file(objectName); // get or open file
        let pointer = file.as_ref().unwrap();
        Ok(drop(pointer))
    }

    fn persistObject(&self, objectName: String) -> Result<(), Error> {
        // let file = self.get_file(objectName.clone()).unwrap(); // get or open file
        // let mut_pointer = file
        //     .into_mut_mapping(0, self.get_size(objectName).unwrap() as usize)
        //     .map_err(|(e, _)| e)
        //     .unwrap();
        let file = self.get_file(objectName); // get or open file
        file.flush();
        Ok(())
    }
}

impl<'a> PartialAccessObjectStorage for FileBackend {

    fn readPartial(&self, objectName: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0_u8; length as usize];
        let file = self.get_file(objectName).unwrap(); // get or open file
        let map = &file.map(offset, length).unwrap();
        buffer.copy_from_slice(map.as_ref());
        Ok(buffer)
    }

    fn writePartial(&self, objectName: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        let mut file = self.get_file(objectName).as_ref(); // get or open file
        let mut mut_pointer = &file
            .unwrap()
            .into_mut_mapping(offset, length)
            .map_err(|(e, _)| e)
            .unwrap();
        mut_pointer.copy_from_slice(&data);

        Ok(length)
    }
}

impl StreamingObjectStorage for FileBackend {}
impl StreamingPartialAccessObjectStorage for FileBackend {}

impl ObjectStorage for FileBackend {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_backend_get_file() {
        let filesystem = FileBackend {
            folder_path: String::from("alpine"),
            ..FileBackend::default()
        };
        let file = filesystem.get_file(String::from("alpine"));
        assert!(file.is_ok());
        drop(file);
    }

    #[test]
    fn test_file_backend_mmap() {
        let filesystem = FileBackend {
            folder_path: String::from("alpine"),
            ..FileBackend::default()
        };
        let mmapped_file = filesystem.mmap_file(String::from("alpine"));
        assert!(mmapped_file.is_ok());
    }

    #[test]
    fn test_file_backend_init() {
        let mut filesystem = FileBackend::default();
        assert!(&filesystem.folder_path == "");
        filesystem.init(String::from("alpine"));
        assert!(&filesystem.folder_path == "alpine");
    }

    #[test]
    fn test_file_backend_startOperationsOnObject() {
        let mut filesystem = FileBackend::default();
        filesystem.startOperationsOnObject(String::from("alpine"));
        assert!(filesystem.open_files.len() == 1);
    }

    #[test]
    fn test_file_backend_endOperationsOnObject() {
        let mut filesystem = FileBackend::default();
        filesystem.startOperationsOnObject(String::from("alpine"));
        assert!(filesystem.open_files.len() == 1);
        filesystem.endOperationsOnObject(String::from("alpine"));
        assert!(filesystem.open_files.len() == 0);
    }
}
