use std::{
    io::{Error},
};

mod config;
pub use self::config::block_storage_with_config;

mod raw;
pub use self::raw::RawBlock;

mod sharded;
pub use self::sharded::ShardedBlock;

use crate::util::Propagation;

pub trait BlockStorage {
    fn init(&mut self);
    fn get_name(&self) -> String;
    fn get_volume_size(&self) -> u64;
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, Error>;
    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error>;
    fn flush(&mut self, offset: u64, length: usize) -> Result<Propagation, Error>;
    fn trim(&mut self, offset: u64, length: usize) -> Result<Propagation, Error>;
    fn close(&mut self);
}
