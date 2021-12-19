use std::{
    str,
    io::{Error},
};

use log;

use crate::{
    object::{ObjectStorage, object_storage_with_config},
    block::{BlockStorage},
};
use crate::util::Propagation;

// Driver: ShardedBlock

pub struct ShardedBlock {
    name: String,
    volume_size: u64,
    shard_size: u64,
    object_storage: Box<dyn ObjectStorage>,
}

impl ShardedBlock {
    pub fn new(name: String, config: String) -> ShardedBlock {
        // TODO: Allow configuring disk size in config string
        //       or a setting like `create=true`
        // TODO: Allow configuring shard size in config string
        let default_shard_size: u64 = 4 * 1024 * 1024;
        let mut sharded_file = ShardedBlock {
            name: name.clone(),
            volume_size: 0_u64,
            shard_size: default_shard_size,
            object_storage: object_storage_with_config(config).unwrap(),
        };
        sharded_file.init();
        sharded_file
    }

    pub fn shard_index(&self, offset: u64) -> usize {
        (offset / &self.shard_size) as usize
    }

    pub fn size_of_volume(&self) -> u64 {
        let object_name = String::from("size");
        let filedata = self.object_storage.read(object_name); // TODO: Errors?
        if filedata.is_err() {
            return 4 * 1024 * 1024 * 1024; // 4 GiB
        }
        // TODO: Allow file to not exist, create if does not exist
        let mut string = str::from_utf8(&filedata.unwrap()).unwrap().to_string();
        string.retain(|c| !c.is_whitespace());
        let volume_size: u64 = string.parse().unwrap();
        volume_size
    }

    pub fn shard_name(&self, index: usize) -> String {
        format!("block-{}", index).to_string()
    }
}

impl BlockStorage for ShardedBlock {
    fn init(&mut self) {
        self.volume_size = self.size_of_volume();
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_volume_size(&self) -> u64 {
        self.volume_size
    }

    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = Vec::new();
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };

        log::trace!("storage::read(start: {}, end: {})", start, end);
        for i in start..=end {
            log::trace!("storage::read(iteration: {})", i);
            let shard_name = self.shard_name(i);

            if self.object_storage.exists(shard_name.clone())? {
                if i == start {
                    let read_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                    let buf = self.object_storage
                        .partial_read(shard_name.clone(), offset % self.shard_size, read_size)?;
                    buffer.extend_from_slice(&buf);
                    continue;
                }
                if i == end {
                    let read_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                    let buf = self.object_storage
                        .partial_read(shard_name.clone(), 0, read_size)?;
                    buffer.extend_from_slice(&buf);
                    break;
                }
                let buf = self.object_storage
                    .read(shard_name.clone())?;
                buffer.extend_from_slice(&buf);
            } else {
                if i == start {
                    let read_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                    buffer.extend_from_slice(&vec![0_u8; read_size]);
                    continue;
                }
                if i == end {
                    let read_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                    buffer.extend_from_slice(&vec![0_u8; read_size]);
                    break;
                }
                buffer.extend_from_slice(&vec![0_u8; self.shard_size as usize]);
            }
        }
        Ok(buffer)
    }

    fn write(&mut self, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error> {
        // let start = self.shard_index(offset);
        // let end = if 0 == (offset + length as u64) % self.shard_size {
        //     self.shard_index(offset + length as u64) - 1
        // } else {
        //     self.shard_index(offset + length as u64)
        // };
        log::trace!("storage::write(offset: {}, length: {})", offset, length);
        let mut overall_propagation : Propagation = Propagation::Guaranteed;

        let mut cur_offset: usize = offset as usize;
        let mut cur_shard;
        let mut written: usize = 0;
        while written < length {
            cur_shard = self.shard_index(cur_offset as u64);
            let shard_offset: usize = cur_offset % self.shard_size as usize;

            // until which byte we will write inside this shard
            let write_target = std::cmp::min(shard_offset + (length - written), self.shard_size as usize);
            log::trace!("write_target {} - shard_offset {}", write_target, shard_offset);
            let write_len: usize = write_target - shard_offset;

            log::trace!("storage::write(shard: {}, offset: {}, len: {})", cur_shard, shard_offset, write_len);
            let shard_name = self.shard_name(cur_shard);

            let slice = &data[written..(written + write_len)];
            let propagated;

            // full write
            if write_len == self.shard_size as usize {
                propagated = self.object_storage.write(shard_name.clone(), slice)?;
            }

            // new object
            else if !self.object_storage.exists(shard_name.clone())? {
                let mut buffer: Vec<u8> = Vec::new();
                // pad zeroes (head)
                if shard_offset > 0 {
                    let head_zeroes: Vec<u8> = vec![0_u8; shard_offset as usize];
                    buffer.extend_from_slice(&head_zeroes);
                }
                buffer.extend_from_slice(slice);
                // pad zeroes (tail)
                if write_target < self.shard_size as usize - 1 {
                    let tail_zeroes: Vec<u8> = vec![0_u8; (self.shard_size as usize - write_len - shard_offset) as usize];
                    buffer.extend_from_slice(&tail_zeroes);
                }
                propagated = self.object_storage.write(shard_name.clone(), &buffer)?;

            // existing object, partial write
            } else {
                propagated = self.object_storage.partial_write(shard_name.clone(), shard_offset as u64, write_len, slice)?;
            }

            written += write_len;
            cur_offset += written;
            if (propagated as u8) >= (Propagation::Queued as u8) {
                log::debug!("storage::write(iteration: {}, {})", cur_shard, propagated as u8);
            } else {
                log::trace!("storage::write(iteration: {}, {})", cur_shard, propagated as u8);
            }
            if (propagated as u8) < (overall_propagation as u8) {
                overall_propagation = propagated;
            }
        }

        Ok(overall_propagation)
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<Propagation, Error> {
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };

        log::debug!("storage::flush(start: {}, end: {})", start, end);
        let mut overall_propagation : Propagation = Propagation::Guaranteed;
        for i in start..=end {
            let shard_name = self.shard_name(i);
            let propagated = self.object_storage.persist_object(shard_name.clone())?;
            if (propagated as u8) >= (Propagation::Queued as u8) {
                log::debug!("storage::flush(iteration: {}, {})", i, propagated as u8);
            } else {
                log::trace!("storage::flush(iteration: {}, {})", i, propagated as u8);
            }
            if (propagated as u8) < (overall_propagation as u8) {
                overall_propagation = propagated;
            }
        }

        Ok(overall_propagation)
    }

    fn trim(&mut self, offset: u64, length: usize) -> Result<Propagation, Error> {
        // TODO: Write unit tests to ensure correct behavior
        // We MUST NOT TRIM any shared that is not requested to be trimmed as a whole
        // So:
        // * start = ceil(offset / self.shard_size)
        // *   end = floor((offset+length) / self.shard_size)
        //
        // TODO: Even the documented logic above needs unit test validation for correctness
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };
        log::debug!("storage::trim(start: {}, end: {})", start, end);
        let mut overall_propagation : Propagation = Propagation::Guaranteed;
        for i in start..=end {
            let object_name = self.shard_name(self.shard_index(offset));
            overall_propagation = self.object_storage.delete_object(object_name)?;
        }
        Ok(overall_propagation)
    }

    fn close(&mut self) {
        log::debug!("storage::close");
        self.object_storage.close();
    }
}
