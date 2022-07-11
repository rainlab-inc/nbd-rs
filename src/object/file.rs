use std::{
    fs::{File, OpenOptions, remove_file},
    io::{Read, Write, Seek, SeekFrom, Error, ErrorKind},
    collections::{HashMap},
    sync::{Arc,RwLock},
    path::{Path,PathBuf},
    ffi::{CString},
    mem::{MaybeUninit},
    os::unix::io::{AsRawFd},
};

use mmap_safe::{MappedFile};
extern crate libc;

use crate::object::{
    ObjectStorage,
    SimpleObjectStorage,
    PartialAccessObjectStorage,
    StreamingObjectStorage,
    StreamingPartialAccessObjectStorage,
    ObjectMeta,
};
use crate::util::Propagation;

pub struct FileBackend {
    folder_path: String,
    open_files: RwLock<HashMap<String, Arc<RwLock<MappedFile>>>>,
}

impl Default for FileBackend {
    fn default() -> FileBackend {
        FileBackend::new(String::from(""))
    }
}

impl FileBackend {
    pub fn new(config: String) -> FileBackend {
        log::debug!("FileBackend.config: {:?}", &config);
        FileBackend {
            folder_path: config.clone(),
            open_files: RwLock::<HashMap<String, Arc<RwLock<MappedFile>>>>::new(
                HashMap::<String, Arc<RwLock<MappedFile>>>::new()
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

    fn get_file(&self, object_name: String) -> Result<Arc<RwLock<MappedFile>>, Error> {
        // TODO: Check if self.openFiles already has the file, return that
        //let file = self.open_file(object_name, false);
        let mut open_files = self.open_files.write().unwrap();
        let mapped_file = open_files.get_key_value(&object_name);
        if mapped_file.is_some() {
            return Ok(Arc::clone(&mapped_file.unwrap().1));
        }

        let mapped_refcell = Arc::new(RwLock::new(MappedFile::open(self.obj_path(object_name.clone()))?));
        let mapped = mapped_refcell.clone();
        open_files.insert(object_name.clone(), mapped_refcell);
        Ok(mapped)
    }

    fn obj_path(&self, object_name: String) -> PathBuf {
        Path::new(&self.folder_path).join(object_name.clone())
    }

    fn get_files_inside_folder(&self, path: PathBuf) -> Result<Vec<ObjectMeta>, Error> {
        let mut files = Vec::new();
        let paths = path.read_dir()?;
        for file in paths {
            let file = file?;
            if file.file_type().unwrap().is_dir() {
                let mut folder_vec =  self.get_files_inside_folder(file.path())?;
                files.append(&mut folder_vec);
            } else {
                let path = file.path().into_os_string().into_string().unwrap().split_once(self.folder_path.as_str()).unwrap().1.to_string();
                let obj = ObjectMeta {
                    path, 
                    size: file.metadata()?.len(),
                };
                files.push(obj);
            }
        }
        Ok(files)
    }
}

impl SimpleObjectStorage for FileBackend {
    fn init(&mut self, conn_str: String) {
        self.folder_path = conn_str.clone()
    }
    
    fn create_object(&self, object_name: String, len: u64) -> Result<(), Error> {
        let path = self.obj_path(object_name.clone());
        let mut file = File::create(path)?;
        file.seek(SeekFrom::Start(len - 1))?;
        file.write_all(&[0_u8])?;
        Ok(())
    }
    
    fn exists(&self, object_name: String) -> Result<bool, Error> {
        let path = self.obj_path(object_name.clone());
        return Ok(path.is_file() && path.exists())
    }

    fn supports_trim(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            let path = if &self.folder_path != "" {
                self.folder_path.clone()
            } else {
                String::from("/")
            };
            let ptr = match CString::new(path.clone()) {
                Ok(p) => p.into_raw(),
                Err(e) => panic!("Error while creating CString of path: {:?}. Full error: {}", path.clone(), e)
            };
            let uninit: MaybeUninit<libc::statfs> = MaybeUninit::uninit();
            unsafe {
                let sfs = &mut uninit.assume_init();
                let result = libc::statfs(ptr, sfs);
                if result != 0 {
                    log::warn!("Error on path: {:?}. Full error: {:?}", path.clone(), Error::last_os_error());
                    return false
                }
                match sfs.f_type {
                    libc::EXT4_SUPER_MAGIC | libc::BTRFS_SUPER_MAGIC | libc::XFS_SUPER_MAGIC | libc::TMPFS_MAGIC => return true,
                    _ => {
                        log::debug!("Type of the filesystem is not one of: EXT4 | BTRFS | XFS | TMPFS!");
                        return false
                    }
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        return false;
    }
    
    fn supports_random_write_access(&self) -> bool {
        true
    }

    fn read(&self, object_name: String) -> Result<Vec<u8>, Error> {
        let open_files = self.open_files.read().unwrap();
        match open_files.get_key_value(&object_name) {
            Some(mapped_file) => {
                let mut buffer: Vec<u8> = Vec::new();
                let mapped_file_ptr = mapped_file.1.read().unwrap();
                let sub = mapped_file_ptr
                    .map(0, mapped_file_ptr.size() as usize)
                    .unwrap();
                buffer.copy_from_slice(sub.as_ref());
                return Ok(buffer)
            },
            None => {
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
                return Ok(buffer)
            }
        }
    }

    fn write(&self, object_name: String, data: &[u8]) -> Result<Propagation, Error> {
        let open_files = self.open_files.read().unwrap();
        match open_files.get_key_value(&object_name) {
            Some(mapped_file) => {
                let mut mapped_file_ptr = mapped_file.1.write().unwrap();
                let size: usize = mapped_file_ptr.size() as usize;
                let mut mut_pointer = mapped_file_ptr
                    .map_mut(0, size)
                    .unwrap();
                mut_pointer.copy_from_slice(&data);
            },
            None => {
                let path = self.obj_path(object_name.clone());
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(path)?;

                file.write_all(data)?;
            }
        }
        // TODO: Consider file.sync_all()? or file.sync_data()?;
        //       or at least to it async ? instead of dropping the file
        Ok(Propagation::Complete)
    }

    fn delete(&self, object_name: String) -> Result<Propagation, Error> {
        remove_file(self.obj_path(object_name))?;
        Ok(Propagation::Guaranteed)
    }

    fn get_size(&self, object_name: String) -> Result<u64, Error> {
        let path = self.obj_path(object_name.clone());
        log::debug!("Getting size of {:?}", path);

        let length_data = path
            .metadata()
            .expect(&format!("Error on getting size of: <{}>", object_name.clone()));

        Ok(length_data.len())
    }
    
    fn get_object_list(&self) -> Result<Vec<ObjectMeta>, Error> {
        self.get_object_list_with_prefix("".to_string())
    }
    
    fn get_object_list_with_prefix(&self, prefix: String) -> Result<Vec<ObjectMeta>, Error> {
        // TODO: Change this to something like grep
        let path = self.obj_path("".to_string());
        let files = self.get_files_inside_folder(path);
        
        match files {
            Err(e) => {
                return Err(e); 
            },
            Ok(files) => {
                let files: Vec<ObjectMeta> = files.into_iter().filter(|x| x.path.starts_with(prefix.as_str())).collect();
                return Ok(files);
            }
        }
    }

    fn destroy(&self) {
        let list = self.get_object_list().unwrap();
        for item in list {
            self.delete(item.path);
        }
    }

    fn start_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        self.get_file(object_name);
        //let mut open_files = self.open_files.write().unwrap();
        // TODO: Check if self.openFiles already has same file, use Rc.increment_strong_count in that case
        // TODO: Mmap? MappedFile::new(f).expect("Something went wrong");
        // TODO: Exact same behavior with `get_file`?
        /*open_files.insert(
            object_name.clone(),
            Arc::new(RwLock::new(MappedFile::open(object_name.clone()).unwrap()))
        );*/
        Ok(())
    }

    fn end_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // TODO: code below is stupid here. just remove file from this.openFiles
        let file = self.get_file(object_name.clone()).unwrap(); // get or open file
        if !(Arc::strong_count(&file) >= 1) {
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#safety-3
            return Err(Error::new(ErrorKind::Other, "Unsafe ending operations on object. There is no operations."))
        }
        unsafe{ Arc::decrement_strong_count(Arc::into_raw(file)); }
        Ok(())
        //Ok(drop(file)) // !?
    }

    fn persist_object(&self, object_name: String) -> Result<Propagation, Error> {
        unsafe{ libc::sync(); }
        // This is only relevant for already open files.

        // let file_refcell = self.get_file(object_name.clone()).unwrap();
        // let mut mut_pointer = file_refcell
        //     .write().unwrap()
        //     .into_mut_mapping(0, self.get_size(object_name.clone()).unwrap() as usize)
        //     .map_err(|(e, _)| e)
        //     .unwrap();
        // mut_pointer.flush();
        Ok(Propagation::Guaranteed)
    }

    fn trim_object (&self, object_name: String, offset: u64, length: usize) -> Result<Propagation, Error> { //hints fallocate
        #[cfg(target_os = "linux")]
        {
            let mut open_files = self.open_files.write().unwrap();
            let mmap_file = open_files.remove_entry(&object_name);
            let path = self.obj_path(object_name.clone());
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(path)?;
            unsafe { libc::fallocate(
                file.as_raw_fd(),
                libc::FALLOC_FL_KEEP_SIZE + libc::FALLOC_FL_PUNCH_HOLE,
                offset as libc::off_t,
                length as libc::off_t
            ); }
            Ok(Propagation::Guaranteed)
        }
        #[cfg(not(target_os = "linux"))]
        Err(Error::new(ErrorKind::Unsupported, "Trim Not Supported"))
    }

    fn close(&mut self) {
        log::debug!("object::file::close");
    }
}

impl PartialAccessObjectStorage for FileBackend {

    fn partial_read(&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        // TODO: Use MMAP if file is already open and mmap'ed.
        let open_files = self.open_files.read().unwrap();
        match open_files.get_key_value(&object_name) {
            Some(mapped_file) => {
                let mut buffer = vec![0_u8; length as usize];
                let map = mapped_file.1.read().unwrap();
                let sub = map.map(offset, length).unwrap();
                buffer.copy_from_slice(sub.as_ref());
                return Ok(buffer)
            },
            None => {
                let path = self.obj_path(object_name.clone());
                let mut buffer: Vec<u8> = Vec::new();
                if !self.exists(object_name.clone())? {
                    return Err(Error::new(ErrorKind::NotFound, "Object Not Found"))
                }

                let mut file = OpenOptions::new()
                    .read(true)
                    .open(path)
                    .unwrap();

                file.seek(SeekFrom::Start(offset))?;
                let mut handle = file.take(length as u64);
                handle.read_to_end(&mut buffer)?;
                return Ok(buffer)
            }
        }
    }

    fn partial_write(&self, object_name: String, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error> {
        let open_files = self.open_files.read().unwrap();
        match open_files.get_key_value(&object_name) {
            Some(mapped_file) => {
                let mut mapped_file_ptr = mapped_file.1.write().unwrap();
                let size = mapped_file_ptr.size() as usize;
                let mut mut_pointer = mapped_file_ptr
                    .map_mut(offset, length)
                    .unwrap();
                mut_pointer.copy_from_slice(&data);
            },
            None => {
                let path = self.obj_path(object_name.clone());
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(path)
                    .unwrap();
                file.seek(SeekFrom::Start(offset))?;
                file.write_all(data)?;
            }
        }
        // TODO: Consider file.sync_all()? or file.sync_data()?;
        //       or at least to it async ? instead of dropping the file
        Ok(Propagation::Complete)
    }
}

impl StreamingObjectStorage for FileBackend {}
impl StreamingPartialAccessObjectStorage for FileBackend {}

impl ObjectStorage for FileBackend {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{OpenOptions},
        io::{Write},
        path::{Path},
    };
    use crate::util::test_utils::TempFolder;

    #[test]
    fn test_file_backend_get_file() {
        let folder = TempFolder::new();
        let dummy_file_name = String::from("dummy_file");
        let mut dummy_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(Path::new(&folder.path).join(dummy_file_name.clone()))
                            .unwrap();
        dummy_file.write(&[0_u8; 1024]);
        let filesystem = FileBackend {
            folder_path: folder.path.clone(),
            ..FileBackend::default()
        };

        let mapped_file = filesystem.get_file(dummy_file_name.clone());
        assert!(mapped_file.is_ok());
        let mapped_file_2 = filesystem.get_file(dummy_file_name.clone());
        assert!(mapped_file_2.is_ok());
    }

    #[test]
    fn test_file_backend_init() {
        let folder = TempFolder::new();
        let dummy_file_name = String::from("dummy_file");
        let mut dummy_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(Path::new(&folder.path).join(dummy_file_name.clone()))
                            .unwrap();
        dummy_file.write(&[0_u8; 1024]);
        let mut filesystem = FileBackend::default();

        assert!(&filesystem.folder_path == "");
        filesystem.init(folder.path.clone());
        assert!(&filesystem.folder_path == &folder.path);
    }

    #[test]
    fn test_file_backend_start_operations_on_object() {
        let folder = TempFolder::new();
        let dummy_file_name = String::from("dummy_file");
        let mut dummy_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(Path::new(&folder.path).join(dummy_file_name.clone()))
                            .unwrap();
        dummy_file.write(&[0_u8; 1024]);
        let filesystem = FileBackend {
            folder_path: folder.path.clone(),
            ..FileBackend::default()
        };

        filesystem.start_operations_on_object(dummy_file_name.clone());
    }

    #[test]
    fn test_file_backend_end_operations_on_object() {
        let folder = TempFolder::new();
        let dummy_file_name = String::from("dummy_file");
        let mut dummy_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(Path::new(&folder.path).join(dummy_file_name.clone()))
                            .unwrap();
        dummy_file.write(&[0_u8; 1024]);
        let filesystem = FileBackend {
            folder_path: folder.path.clone(),
            ..FileBackend::default()
        };

        filesystem.start_operations_on_object(dummy_file_name.clone());
        filesystem.end_operations_on_object(dummy_file_name.clone());
    }
}
