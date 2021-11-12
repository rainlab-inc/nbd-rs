use std::{
    fs::{File, OpenOptions},
    io::{Read, Write, Seek, SeekFrom, Error, ErrorKind},
    collections::{HashMap},
    rc::{Rc},
    cell::{RefCell},
    path::{Path,PathBuf},
};

use mmap_safe::{MappedFile};

use crate::object::{
    ObjectStorage,
    SimpleObjectStorage,
    PartialAccessObjectStorage,
    StreamingObjectStorage,
    StreamingPartialAccessObjectStorage,
};

extern crate libc;

pub struct FileBackend {
    folder_path: String,
    open_files: RefCell<HashMap<String, Rc<RefCell<MappedFile>>>>,
}

impl Default for FileBackend {
    fn default() -> FileBackend {
        FileBackend::new(String::from(""))
    }
}

impl FileBackend {
    fn print(&self) {
        let open_files = self.open_files.borrow();
        println!("{}", format!("Folder Path: '{path}', Keys: {files:?}", path=self.folder_path, files=open_files.keys()));
    }

    pub fn new(config: String) -> FileBackend {
        FileBackend {
            folder_path: config.clone(),
            open_files: RefCell::<HashMap<String, Rc<RefCell<MappedFile>>>>::new(
                HashMap::<String, Rc<RefCell<MappedFile>>>::new()
            )
        }
    }

    fn open_file(&self, object_name: String, create: bool) -> Result<File, Error> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(create)
            .open(object_name.clone())
    }

    fn get_file(&self, object_name: String) -> Result<Rc<RefCell<MappedFile>>, Error> {
        // TODO: Check if self.openFiles already has the file, return that
        //let file = self.open_file(object_name, false);
        let mut open_files = self.open_files.borrow_mut();
        let mapped_file = open_files.get_key_value(&object_name);
        if mapped_file.is_some() {
            return Ok(Rc::clone(&mapped_file.unwrap().1));
        }

        let mapped_refcell = Rc::new(RefCell::new(MappedFile::open(object_name.clone()).unwrap()));
        let mapped = mapped_refcell.clone();
        open_files.insert(object_name.clone(), mapped_refcell);
        Ok(mapped)
    }

    fn obj_path(&self, object_name: String) -> PathBuf {
        Path::new(&self.folder_path).join(object_name.clone())
    }
}

impl<'a> SimpleObjectStorage for FileBackend {
    fn init(&mut self, conn_str: String) {
        self.folder_path = conn_str.clone()
    }

    fn exists(&self, object_name: String) -> Result<bool, Error> {
        let path = self.obj_path(object_name.clone());
        return Ok(path.is_file() && path.exists())
    }

    fn read(&self, object_name: String) -> Result<Vec<u8>, Error> {
        let path = self.obj_path(object_name.clone());
        let mut buffer: Vec<u8> = Vec::new();
        if !self.exists(object_name.clone())? {
            return Err(Error::new(ErrorKind::NotFound, "Object Not Found"))
        }

        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .unwrap();

        file
            .read_to_end(&mut buffer)
            .expect(&format!("couldn't read object: {:?}", object_name.clone()));

        Ok(buffer)
    }

    fn write(&self, object_name: String, data: &[u8]) -> Result<(), Error> {
        let path = self.obj_path(object_name.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        file.write_all(data)?;
        // TODO: Consider file.sync_all()? or file.sync_data()?;
        //       or at least to it async ? instead of dropping the file
        Ok(())
    }

    fn delete(&self, object_name: String) -> Result<(), Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    fn get_size(&self, object_name: String) -> Result<u64, Error> {
        let path = self.obj_path(object_name.clone());

        let length_data = path
            .metadata()
            .expect(&format!("Error on getting size of: <{}>", object_name.clone()));

        Ok(length_data.len())
    }

    fn start_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        self.get_file(object_name);
        //let mut open_files = self.open_files.borrow_mut();
        // TODO: Check if self.openFiles already has same file, use Rc.increment_strong_count in that case
        // TODO: Mmap? MappedFile::new(f).expect("Something went wrong");
        // TODO: Exact same behavior with `get_file`?
        /*open_files.insert(
            object_name.clone(),
            Rc::new(RefCell::new(MappedFile::open(object_name.clone()).unwrap()))
        );*/
        Ok(())
    }

    fn end_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // TODO: code below is stupid here. just remove file from this.openFiles
        let file = self.get_file(object_name.clone()).unwrap(); // get or open file
        if !(Rc::strong_count(&file) >= 1) {
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#safety-3
            return Err(Error::new(ErrorKind::Other, "Unsafe ending operations on object. There is no operations."))
        }
        unsafe{ Rc::decrement_strong_count(Rc::into_raw(file)); }
        Ok(())
        //Ok(drop(file)) // !?
    }

    fn persist_object(&self, object_name: String) -> Result<(), Error> {
        unsafe{ libc::sync(); }
        // This is only relevant for already open files.

        // let file_refcell = self.get_file(object_name.clone()).unwrap();
        // let mut mut_pointer = file_refcell
        //     .borrow_mut()
        //     .into_mut_mapping(0, self.get_size(object_name.clone()).unwrap() as usize)
        //     .map_err(|(e, _)| e)
        //     .unwrap();
        // mut_pointer.flush();
        Ok(())
    }
}

impl<'a> PartialAccessObjectStorage for FileBackend {

    fn partial_read(&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        // TODO: Use MMAP if file is already open and mmap'ed.
        let open_files = self.open_files.borrow();
        match open_files.get_key_value(&object_name) {
            Some(mapped_file) => {
                let mut buffer = vec![0_u8; length as usize];
                let map = mapped_file.1.borrow();
                let sub = map.map(offset, length).unwrap();
                buffer.copy_from_slice(sub.as_ref());
                return Ok(buffer)
            },
            None => {
                let path = self.obj_path(object_name.clone());
                let mut buffer = vec![0_u8; length];
                if !self.exists(object_name.clone())? {
                    return Err(Error::new(ErrorKind::NotFound, "Object Not Found"))
                }

                let mut file = OpenOptions::new()
                    .read(true)
                    .open(path)
                    .unwrap();

                file.seek(SeekFrom::Start(offset))?;
                file.read_exact(&mut buffer)
                    .expect(&format!("couldn't read object: {:?}", object_name.clone()));
                return Ok(buffer)
            }
        }
    }

    fn partial_write(&self, object_name: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        // TODO: Use MMAP if file is already open and mmap'ed.
        // self.print();
        // let file = self.get_file(object_name.clone()).unwrap(); // get or open file
        // self.print();
        // let mut mut_pointer = file
        //     .into_mut_mapping(offset, length)
        //     .map_err(|(e, _)| e)
        //     .unwrap();
        // mut_pointer.copy_from_slice(&data);
        let path = self.obj_path(object_name.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        file.seek(SeekFrom::Start(offset))?;
        file.write_all(data)?;
        // TODO: Consider file.sync_all()? or file.sync_data()?;
        //       or at least to it async ? instead of dropping the file
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
        let mapped_file = filesystem.get_file(String::from("alpine"));
        assert!(mapped_file.is_ok());
        let mapped_file_2 = filesystem.get_file(String::from("alpine"));
        assert!(mapped_file_2.is_ok());
    }

    #[test]
    fn test_file_backend_init() {
        let mut filesystem = FileBackend::default();
        assert!(&filesystem.folder_path == "");
        filesystem.init(String::from("alpine"));
        assert!(&filesystem.folder_path == "alpine");
    }

    #[test]
    fn test_file_backend_start_operations_on_object() {
        let mut filesystem = FileBackend::default();
        filesystem.start_operations_on_object(String::from("alpine"));
    }

    #[test]
    fn test_file_backend_end_operations_on_object() {
        let mut filesystem = FileBackend::default();
        filesystem.start_operations_on_object(String::from("alpine"));
        filesystem.end_operations_on_object(String::from("alpine"));
    }
}
