use std::{
    io::{Error},
};

mod config;
pub use self::config::block_storage_with_config;

mod raw;
pub use self::raw::RawBlock;

mod sharded;
pub use self::sharded::ShardedBlock;

use crate::util::{Propagation, AlignedBlockIter};

pub trait BlockStorage {
    fn init(&mut self);
    fn get_name(&self) -> String;
    fn get_volume_size(&self) -> u64;
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, Error>;
    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error>;
    fn flush(&mut self, offset: u64, length: usize) -> Result<Propagation, Error>;
    fn close(&mut self);

    // `fill` has a default implementation
    fn fill(&mut self, offset: u64, length: usize, fillbyte: u8) -> Result<Propagation, Error> {
        // Don't allocate too big memory at once
        // Split this into 4MB chunks if bigger than 4M
        // And do it in an aligned fashion.
        let mut overall_propagation : Propagation = Propagation::Guaranteed;
        for r in (AlignedBlockIter{ from: offset as usize, to: offset as usize+length, blksize: 4*1024*1024 }) {
            let filldata: Vec<u8> = vec![fillbyte; r.end - r.start];
            let propagated = self.write(r.start as u64, r.end - r.start, &filldata)?;
            if (propagated as u8) < (overall_propagation as u8) {
                overall_propagation = propagated;
            }
        }
        Ok(overall_propagation)
    }

    // default sub-optimal implementation for `trim`
    fn trim(&mut self, offset: u64, length: usize) -> Result<Propagation, Error> {
        self.fill(offset, length, 0_u8);
        return Ok(Propagation::Unsupported);
    }
}
