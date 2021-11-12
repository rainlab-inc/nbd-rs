use std::{
    io::{Read, Write, Error, ErrorKind},
};

use url::{Url, ParseError};

//mod config;
//pub use self::config::storage_with_config;

mod file;
pub use self::file::FileBackend;

pub trait SimpleObjectStorage {
    fn init     (&mut self, connStr: String);

    // simplest interface
    fn exists   (&self, objectName: String) -> Result<bool, Error>;
    fn get_size (&self, objectName: String) -> Result<u64, Error>;
    fn read     (&self, objectName: String) -> Result<Vec<u8>, Error>;
    fn write    (&self, objectName: String, data: &[u8]) -> Result<(), Error>;
    fn delete   (&self, objectName: String) -> Result<(), Error>;

    // Hint interface (Optional, Default=Noop)
    // hints the object storage backend about long access on object, so the backend can do stuff like MMAP
    fn startOperationsOnObject (&mut self, objectName: String) -> Result<(), Error>; // hints open  (or ++refCount==1?open)
    fn endOperationsOnObject   (&mut self, objectName: String) -> Result<(), Error>; // hints close (or --refCount==0?close)
    fn persistObject           (&mut self, objectName: String) -> Result<(), Error>; // hints flush
}

pub trait PartialAccessObjectStorage {
    // partial reads/writes

    // TODO: these can also have dumb default implementations
    fn readPartial  (&self, objectName: String, offset: u64, length: usize) -> Result<Vec<u8>, Error>;
    fn writePartial (&mut self, objectName: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error>;
}

// With given stream, read `length` bytes, and write to target object, avoids buffering on consumer side
pub trait StreamingObjectStorage {
    // TODO: these can also have dumb default implementations
    fn readIntoStream  (&self, objectName: String, stream: Box<dyn Write>) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn writeFromStream (&self, objectName: String, stream: Box<dyn Read>,  length: usize) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
}

pub trait StreamingPartialAccessObjectStorage {
    // TODO: these can also have dumb default implementations
    fn partialReadIntoStream  (&self, objectName: String, stream: Box<dyn Write>, offset: u64, length: usize) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
    fn partialWriteFromStream (&self, objectName: String, stream: Box<dyn Read>,  offset: u64, length: usize) -> Result<usize, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
}

pub trait ObjectStorage: SimpleObjectStorage + PartialAccessObjectStorage + StreamingObjectStorage + StreamingPartialAccessObjectStorage {}


pub fn storage_with_config(config: String) -> Box<dyn ObjectStorage> {
    let issue_list_url = Url::parse(&config).expect("Could not parse config url.");
    match issue_list_url.scheme() {
        "file" => {
            let object_storage: Box<dyn ObjectStorage> = Box::new(FileBackend {
                folder_path: String::from(issue_list_url.path()),
                ..FileBackend::default()
            });
            return object_storage
        },
        e => {
            panic!("Invalid url scheme: <{}>", e);
        }
    };
}
