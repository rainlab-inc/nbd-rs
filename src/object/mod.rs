use std::{
    io::{Read, Write, Error},
};

pub trait SimpleObjectStorage {
    fn init     (&mut self, connStr: String);

    // simplest interface
    fn exists   (&self, objectName: String) -> bool;
    fn get_size (&self, objectName: String) -> Result<u64, Error>;
    fn read     (&self, objectName: String) -> Result<&[u8], Error>;
    fn write    (&self, objectName: String, data: &[u8]) -> Result<(), Error>;
    fn delete   (&self, objectName: String) -> Result<(), Error>;

    // Hint interface (Optional, Default=Noop)
    // hints the object storage backend about long access on object, so the backend can do stuff like MMAP
    fn startOperationsOnObject (&self, objectName: String) -> Result<(), Error> { Ok(()) } // hints open  (or ++refCount==1?open)
    fn endOperationsOnObject   (&self, objectName: String) -> Result<(), Error> { Ok(()) } // hints close (or --refCount==0?close)
    fn persistObject           (&self, objectName: String) -> Result<(), Error> { Ok(()) } // hints flush
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
    fn readIntoStream  (&self, objectName: String, stream: Box<dyn Write>) -> Result<usize, Error>;
    fn writeFromStream (&self, objectName: String, stream: Box<dyn Read>,  length: usize) -> Result<usize, Error>;
}

pub trait StreamingPartialAccessObjectStorage {
    // TODO: these can also have dumb default implementations
    fn partialReadIntoStream  (&self, objectName: String, stream: Box<dyn Write>, offset: u64, length: usize) -> Result<usize, Error>;
    fn partialWriteFromStream (&self, objectName: String, stream: Box<dyn Read>,  offset: u64, length: usize) -> Result<usize, Error>;
}

pub trait ObjectStorage: SimpleObjectStorage + PartialAccessObjectStorage + StreamingObjectStorage + StreamingPartialAccessObjectStorage {}


pub fn storage_with_config(config: String) -> Result<Box<dyn ObjectStorage>, Error> {
    const storage_type = // TODO: parse config as url
    match storage_type {
        "file" => {
            Ok(())
        },
        _ => {
            // hard fail
        }
    };
}

