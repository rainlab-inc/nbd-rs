use std::{
    io::{Read, Write, Error}
};

pub trait ObjectStorage {
    fn init(&mut self, connStr: String);

    // simplest interface
    fn exists(&self, objectName: String) -> Bool;
    fn read(&self, objectName: String) -> Result<&[u8], Error>;
    fn write(&self, objectName: String, data: &[u8]) -> Result<(), Error>;
    fn delete(&self, objectName: String) -> Result<(), Error>;

    // partial reads/writes
    fn readPartial(&self, objectName: String, offset: u64, length: usize) -> Result<&[u8], Error>;
    fn writePartial(&self, objectName: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error>;

    // With given stream, read `length` bytes, and write to target object, avoids buffering on consumer side
    fn writeFromStream(&self, objectName: String, length: usize, stream: &impl Read) -> Result<usize, Error>;
    fn partialWriteFromStream(&self, objectName: String, offset: u64, length: usize, stream: &impl Read) -> Result<usize, Error>;
    fn readIntoStream(&self, objectName: String, stream: &impl Write) -> Result<usize, Error>;
    fn partialReadIntoStream(&self, objectName: String, offset: u64, length: usize, stream: &impl Write) -> Result<usize, Error>;

    // Hint interface
    // hints the object storage backend about long access on object, so the backend can do stuff like MMAP
    fn startOperationsOnObject(&self, objectName: String) -> Result<(), Error>; // hints open (or ++refCount==1?open)
    fn endOperationsOnObject(&self, objectName: String) -> Result<(), Error>; // hints close (or --refCount==0?close)
    fn persistObject(&self, objectName: String) -> Result<(), Error>; // hints flush
}
