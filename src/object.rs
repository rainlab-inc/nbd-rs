use std::{
    io::{Read, Write, Error},
    std::rc::{Rc}
};

pub trait SimpleObjectStorage {
    fn init(&mut self, connStr: String);

    // simplest interface
    fn exists   (&self, objectName: String) -> Bool;
    fn get_size (&self, objectName: String) -> Result<u64, Error>;
    fn read     (&self, objectName: String) -> Result<&[u8], Error>;
    fn write    (&self, objectName: String, data: &[u8]) -> Result<(), Error>;
    fn delete   (&self, objectName: String) -> Result<(), Error>;

    // Hint interface (Optional, Default=Noop)
    // hints the object storage backend about long access on object, so the backend can do stuff like MMAP
    fn startOperationsOnObject (&self, objectName: String) -> Result<(), Error> { Ok(()) }; // hints open  (or ++refCount==1?open)
    fn endOperationsOnObject   (&self, objectName: String) -> Result<(), Error> { Ok(()) }; // hints close (or --refCount==0?close)
    fn persistObject           (&self, objectName: String) -> Result<(), Error> { Ok(()) }; // hints flush
}

pub trait PartialAccessObjectStorage {
    // partial reads/writes

    // TODO: these can also have dumb default implementations
    fn readPartial  (&self, objectName: String, offset: u64, length: usize) -> Result<&[u8], Error>;
    fn writePartial (&self, objectName: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error>;
}

// With given stream, read `length` bytes, and write to target object, avoids buffering on consumer side
pub trait StreamingObjectStorage {
    // TODO: these can also have dumb default implementations
    fn readIntoStream         (&self, objectName: String, stream: &impl Write) -> Result<usize, Error>;
    fn writeFromStream        (&self, objectName: String, stream: &impl Read,  length: usize) -> Result<usize, Error>;
}

pub trait StreamingPartialAccessObjectStorage {
    // TODO: these can also have dumb default implementations
    fn partialReadIntoStream  (&self, objectName: String, stream: &impl Write, offset: u64, length: usize) -> Result<usize, Error>;
    fn partialWriteFromStream (&self, objectName: String, stream: &impl Read,  offset: u64, length: usize) -> Result<usize, Error>;
}

pub trait ObjectStorage: SimpleObjectStorage + PartialAccessObjectStorage + StreamingObjectStorage + StreamingPartialAccessObjectStorage {}

pub struct FileBackend {
    folderpath: String,
    openFiles: Vec<Rc<MappedFile>>,
}

impl FileBackend {
    fn open_file(&self, objectName: String) -> Result(File, Error) {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(objectName.clone())
            .expect("Unable to open file");

        return f
    }

    fn mmap_file(&self, objectName: String) -> Result(File, Error) {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(objectName.clone())
            .expect("Unable to open file");

        return MappedFile::new(f)
    }

    fn get_file(&self, objectName: String) -> Result(File, Error) {
        // TODO: Check if self.openFiles already has the file, return that
        return self.open_file(objectName);
    }
}

impl<'a> ObjectStorage for FileBackend {
    fn init(&mut self, connStr: String) {
        self.folderPath = connStr.clone()
    }

    fn startOperationsOnObject (&self, objectName: String) -> Result<(), Error> {
        // TODO: Check if self.openFiles already has same file, use Rc.increment_strong_count in that case

        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(objectName.clone())
            .expect("Unable to open file");

        // TODO: Mmap? MappedFile::new(f).expect("Something went wrong");

        self.openFiles.add(Rc.new(MappedFile::new(f)))
    }

    fn endOperationsOnObject(&self, objectName: String) -> Result<(), Error> {
        // TODO: code below is stupid here. just remove file from this.openFiles
        let file = self.get_file(objectName); // get or open file
        let pointer = file.as_ref().unwrap();
        drop(pointer);
    }

    fn persistObject(&self, objectName: String) -> Result<(), Error> {
        let file = self.get_file(objectName); // get or open file
        let mut_pointer = file
            .unwrap()
            .into_mut_mapping(offset, length)
            .map_err(|(e, _)| e)
            .unwrap();
        /*
        let pointer = BorrowMut::<MappedFile>::borrow_mut(&mut self.pointer);
        */
        mut_pointer.flush();
        Ok(())
    }
}

impl<'a> PartialAccessObjectStorage for FileBackend {

    fn readPartial(&self, objectName: String, offset: u64, length: usize) -> Result<&[u8], Error> {
        let mut buffer = vec![0_u8; length as usize];
        let file = self.get_file(objectName); // get or open file
        buffer.copy_from_slice(file.as_ref().unwrap().map(offset, length).unwrap());
        buffer
    }

    fn writePartial(&self, objectName: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        let file = self.get_file(objectName); // get or open file
        let mut mut_pointer = file
            .unwrap()
            .into_mut_mapping(offset, length)
            .map_err(|(e, _)| e)
            .unwrap();
        mut_pointer.copy_from_slice(&data);

        Ok(length)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_backend_get_file() {
        let filesystem = FileBackend {
            folderpath: "test",
            ..Default::<FileBackend>::default()
        };
        let file = filesystem.get_file("test");
        assert!(file.is_ok());
        assert!(file.metadata().unwrap().is_file());
    }

    #[test]
    fn test_file_backend_mmap() {
        let filesystem = FileBackend {
            folderpath: "test",
            ..Default::<FileBackend>::default()
        };
        let mmaped_file = filesystem.mmap_file("test");
        assert!(mmaped_file.is_ok());
    }

    #[test]
    fn test_file_backend_init() {
        let mut filesystem = Default::<FileBackend>::default();
        assert!(&filesystem.name == "");
        filesystem.init(String::from("test"));
        assert!(&filesystem.name == "");
    }

    #[test]
    fn test_file_backend_startOperationsOnObject() {
        let filesystem = Default::<FileBackend>::default();
        filesystem.startOperationsOnObject(String::from("test"))
        assert!(filesystem.open_files.len() == 1);
    }

    #[test]
    fn test_file_backend_endOperationsOnObject() {
        let filesystem = Default::<FileBackend>::default();
        filesystem.startOperationsOnObject(String::from("test"));
        assert!(filesystem.open_files.len() == 1);
        filesystem.endOperationsOnObject(String::from("test"));
        assert!(filesystem.open_files.len() == 0);
    }
}
