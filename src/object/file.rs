use std::{
    fs::{File, OpenOptions},
    io::{Read, Write, Error, ErrorKind},
    rc::{Rc},
    collections::{HashMap},
    cell::{RefCell, RefMut},
    path::{Path},
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
    open_files: RefCell<HashMap<String, RefCell<MappedFile>>>,
}

impl Default for FileBackend {
    fn default() -> FileBackend {
        FileBackend {
            folder_path: String::from(""),
            open_files: RefCell::<HashMap<String, RefCell<MappedFile>>>::new(
                HashMap::<String, RefCell<MappedFile>>::new()
            )
        }
    }
}

impl FileBackend {
    fn print(&self) {
        let open_files = self.open_files.borrow();
        println!("{}", format!("Folder Path: '{path}', Keys: {files:?}", path=self.folder_path, files=open_files.keys()));
    }

    fn open_file(&self, objectName: String, create: bool) -> Result<File, Error> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(create)
            .open(objectName.clone())
    }

    fn mmap_file(&self, objectName: String) -> Result<RefMut<MappedFile>, Error> {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(objectName.clone())
            .expect("Unable to open file");
        let mut open_files = self.open_files.borrow_mut();

        let mapped_file = match open_files.get_key_value(&objectName.clone()) {
            Some(m) => m.1.borrow_mut(),
            None => {
                let mapped = RefCell::new(MappedFile::new(f).unwrap());
                open_files.insert(objectName.clone(), mapped);
                mapped.borrow_mut()
            }
        };
        Ok(mapped_file)
    }

    fn get_file(&self, objectName: String) -> Result<RefMut<MappedFile>, Error> {
        // TODO: Check if self.openFiles already has the file, return that
        //let file = self.open_file(objectName, false);
        self.mmap_file(objectName)
    }

    fn obj_path(&self, objectName: String) -> &Path {
        &Path::new(&self.folder_path).join(objectName.clone())
    }
}

impl<'a> SimpleObjectStorage for FileBackend {
    fn init(&mut self, connStr: String) {
        self.folder_path = connStr.clone()
    }

    fn exists(&self, objectName: String) -> Result<bool, Error> {
        let path = self.obj_path(objectName);
        return Ok(path.is_file() && path.exists())
    }

    fn read(&self, objectName: String) -> Result<Vec<u8>, Error> {
        let path = self.obj_path(objectName);
        let mut buffer: Vec<u8> = Vec::new();
        if !self.exists(objectName)? {
            return Err(Error::new(ErrorKind::NotFound, "Object Not Found"))
        }

        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .unwrap();

        file
            .read_to_end(&mut buffer)
            .expect(&format!("couldn't read object: {:?}", objectName));

        Ok(buffer)
    }

    fn write(&self, objectName: String, data: &[u8]) -> Result<(), Error> {
        let path = self.obj_path(objectName);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        file.write_all(data)?;
        Ok(())
    }

    fn delete(&self, objectName: String) -> Result<(), Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    fn get_size (&self, objectName: String) -> Result<u64, Error> {
        let path = self.obj_path(objectName);

        let length_data = path
            .metadata()
            .expect(&format!("Error on getting size of: <{}>", objectName));

        Ok(length_data.len())
    }

    fn startOperationsOnObject (&self, objectName: String) -> Result<(), Error> {
        let mut open_files = self.open_files.borrow_mut();
        // TODO: Check if self.openFiles already has same file, use Rc.increment_strong_count in that case

        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(objectName.clone())
            .expect("Unable to open file");

        // TODO: Mmap? MappedFile::new(f).expect("Something went wrong");
        open_files.insert(objectName.clone(), RefCell::new(MappedFile::new(f).unwrap()));
        Ok(())
    }

    fn endOperationsOnObject(&self, objectName: String) -> Result<(), Error> {
        // TODO: code below is stupid here. just remove file from this.openFiles
        let file = self.get_file(objectName).unwrap(); // get or open file
        let pointer = file;
        Ok(drop(pointer))
    }

    fn persistObject(&self, objectName: String) -> Result<(), Error> {
        // let file = self.get_file(objectName.clone()).unwrap(); // get or open file
        let file = self.get_file(objectName.clone()).unwrap(); // get or open file
        let mut_pointer = file
            .into_mut_mapping(0, self.get_size(objectName).unwrap() as usize)
            .map_err(|(e, _)| e)
            .unwrap();
        mut_pointer.flush();
        Ok(())
    }
}

impl<'a> PartialAccessObjectStorage for FileBackend {

    fn readPartial(&self, objectName: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0_u8; length as usize];
        let file = self.get_file(objectName).unwrap(); // get or open file
        let map = file.map(offset, length).unwrap();
        buffer.copy_from_slice(map.as_ref());
        Ok(buffer)
    }

    fn writePartial(&self, objectName: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        self.print();
        let file = self.get_file(objectName.clone()).unwrap();
        self.print();
        let mut mut_pointer = file
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
        //assert!(filesystem.open_files.len() == 1);
    }

    #[test]
    fn test_file_backend_endOperationsOnObject() {
        let mut filesystem = FileBackend::default();
        filesystem.startOperationsOnObject(String::from("alpine"));
        //assert!(filesystem.open_files.len() == 1);
        filesystem.endOperationsOnObject(String::from("alpine"));
        //assert!(filesystem.open_files.len() == 0);
    }
}
