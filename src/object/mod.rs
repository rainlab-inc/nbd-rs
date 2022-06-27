use std::{
    io::{Read, Write, Error, ErrorKind},
};

mod config;
pub use self::config::object_storage_with_config;
pub use self::config::object_storages_with_config;

mod file;
pub use self::file::FileBackend;

mod s3;
pub use self::s3::S3Backend;

mod cache;
pub use self::cache::CacheBackend;

use crate::util::Propagation;

pub trait SimpleObjectStorage {
    fn init     (&mut self, conn_str: String);

    // simplest interface
    fn exists   (&self, object_name: String) -> Result<bool, Error>;
    fn get_size (&self, object_name: String) -> Result<u64, Error>;
    fn get_object_list(&self) -> Result<Vec<ObjectMeta>, Error>;
    fn get_object_list_with_prefix(&self, prefix: String) -> Result<Vec<ObjectMeta>, Error>;
    fn supports_trim(&self) -> bool {
        false
    }
    fn supports_random_write_access(&self) -> bool;
    fn read     (&self, object_name: String) -> Result<Vec<u8>, Error>;
    fn write    (&self, object_name: String, data: &[u8]) -> Result<Propagation, Error>;
    fn delete   (&self, object_name: String) -> Result<Propagation, Error>;

    // Hint interface (Optional, Default=Noop)
    // hints the object storage backend about long access on object, so the backend can do stuff like MMAP
    fn start_operations_on_object (&self, object_name: String) -> Result<(), Error>; // hints open  (or ++refCount==1?open)
    fn end_operations_on_object   (&self, object_name: String) -> Result<(), Error>; // hints close (or --refCount==0?close)
    fn persist_object             (&self, object_name: String) -> Result<Propagation, Error>; // hints flush
    fn trim_object                (&self, object_name: String, offset: u64, length: usize) -> Result<Propagation, Error> { //hints fallocate
        Err(Error::new(ErrorKind::Unsupported, "Trim Not Supported"))
    }
    fn close                      (&mut self);
}

pub trait PartialAccessObjectStorage {
    // partial reads/writes

    // TODO: these can also have dumb default implementations
    fn partial_read  (&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error>;
    fn partial_write (&self, object_name: String, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error>;
}

// With given stream, read `length` bytes, and write to target object, avoids buffering on consumer side
pub trait StreamingObjectStorage {
    // TODO: these can also have dumb default implementations
    fn read_into  (&self, object_name: String, stream: Box<dyn Write>) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn write_from (&self, object_name: String, stream: Box<dyn Read>,  length: usize) -> Result<Propagation, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
}

pub trait StreamingPartialAccessObjectStorage {
    // TODO: these can also have dumb default implementations
    fn partial_read_into  (&self, object_name: String, stream: Box<dyn Write>, offset: u64, length: usize) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn partial_write_from (&self, object_name: String, stream: Box<dyn Read>,  offset: u64, length: usize) -> Result<Propagation, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
}

pub trait ObjectStorage: SimpleObjectStorage + PartialAccessObjectStorage + StreamingObjectStorage + StreamingPartialAccessObjectStorage + Send {}

#[derive(Debug)]
pub struct ObjectMeta {
    pub path: String,
    pub size: u64
}
