use std::{
    io::{Read, Write, Error, ErrorKind},
};

mod config;
pub use self::config::storage_with_config;

mod file;
pub use self::file::FileBackend;

pub trait SimpleObjectStorage {
    fn init     (&mut self, conn_str: String);

    // simplest interface
    fn exists   (&self, object_name: String) -> Result<bool, Error>;
    fn get_size (&self, object_name: String) -> Result<u64, Error>;
    fn read     (&self, object_name: String) -> Result<Vec<u8>, Error>;
    fn write    (&self, object_name: String, data: &[u8]) -> Result<(), Error>;
    fn delete   (&self, object_name: String) -> Result<(), Error>;

    // Hint interface (Optional, Default=Noop)
    // hints the object storage backend about long access on object, so the backend can do stuff like MMAP
    fn startOperationsOnObject (&self, object_name: String) -> Result<(), Error>; // hints open  (or ++refCount==1?open)
    fn endOperationsOnObject   (&self, object_name: String) -> Result<(), Error>; // hints close (or --refCount==0?close)
    fn persistObject           (&self, object_name: String) -> Result<(), Error>; // hints flush
}

pub trait PartialAccessObjectStorage {
    // partial reads/writes

    // TODO: these can also have dumb default implementations
    fn readPartial  (&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error>;
    fn writePartial (&self, object_name: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error>;
}

// With given stream, read `length` bytes, and write to target object, avoids buffering on consumer side
pub trait StreamingObjectStorage {
    // TODO: these can also have dumb default implementations
    fn readIntoStream  (&self, object_name: String, stream: Box<dyn Write>) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn writeFromStream (&self, object_name: String, stream: Box<dyn Read>,  length: usize) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
}

pub trait StreamingPartialAccessObjectStorage {
    // TODO: these can also have dumb default implementations
    fn partialReadIntoStream  (&self, object_name: String, stream: Box<dyn Write>, offset: u64, length: usize) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn partialWriteFromStream (&self, object_name: String, stream: Box<dyn Read>,  offset: u64, length: usize) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
}

pub trait ObjectStorage: SimpleObjectStorage + PartialAccessObjectStorage + StreamingObjectStorage + StreamingPartialAccessObjectStorage {}
