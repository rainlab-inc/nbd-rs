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

    fn supports_trim(&self) -> bool {
        true
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
                    let mut read_size = ((length as u64 + offset) % self.shard_size) as usize;
                    if read_size == 0 {
                        read_size = self.shard_size as usize;
                    }
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
            cur_offset += write_len;
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
        let start = self.shard_index(offset);
        let end = if 0 == (offset + length as u64) % self.shard_size {
            self.shard_index(offset + length as u64) - 1
        } else {
            self.shard_index(offset + length as u64)
        };
        log::debug!("storage::trim(start: {}, end: {})", start, end);
        let mut overall_propagation : Propagation = Propagation::Guaranteed;
        for i in start..=end {
            let object_name = self.shard_name(i);
            if i == start {
                let trim_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                if trim_size as u64 % self.shard_size == 0 {
                    overall_propagation = self.object_storage.delete(object_name)?;
                } else {
                    overall_propagation = self.object_storage.partial_write(
                        object_name,
                        offset % self.shard_size,
                        trim_size,
                        &vec![0_u8; trim_size]
                    )?;
                }
            } else if i == end {
                let trim_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                if trim_size as u64 % self.shard_size == 0 {
                    overall_propagation = self.object_storage.delete(object_name)?;
                } else {
                    overall_propagation = self.object_storage.partial_write(
                        object_name,
                        0,
                        trim_size,
                        &vec![0_u8; trim_size]
                    )?;
                }
            } else {
                overall_propagation = self.object_storage.delete(object_name)?;
            }
        }
        Ok(overall_propagation)
    }

    fn close(&mut self) {
        log::debug!("storage::close");
        self.object_storage.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        time::{SystemTime, UNIX_EPOCH},
        fs::{OpenOptions, create_dir, remove_dir_all},
        ffi::{CString},
        io::{Write},
        path::{Path},
    };
    extern crate libc;

    struct TempFolder {
        path: String
    }

    impl TempFolder {
        fn new() -> TempFolder {
            let ptr = CString::new("__test_nbd_rs_sharded_file_trim_accuracy_XXXXXX")
                        .unwrap()
                        .into_raw();
            unsafe { let folder = libc::mkdtemp(ptr); }
            let path = unsafe { CString::from_raw(ptr) }.into_string().unwrap();
            TempFolder { path: path }
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            remove_dir_all(self.path.clone());
        }
    }

    #[test]
    fn test_sharded_block_file_object_trim_case_1() {
        // Case 1:
        // Trim range contains the first, the last, and the intermediary shards results in deletion
        // of all of the contained shards.
        let folder = TempFolder::new();
        let mut size_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(format!("{}/size", folder.path.clone()))
                            .unwrap();
        size_file.write(b"16777216");
        let mut sharded_block = ShardedBlock::new(
            String::from("test"),
            format!("file:///{}", folder.path.clone())
        );
        assert!(sharded_block.size_of_volume() == 16 * 1024 * 1024);
        sharded_block.write(0_u64, 16 * 1024 * 1024 as usize, &[1_u8; 16 * 1024 * 1024]);

        sharded_block.trim(0_u64, 12 * 1024 * 1024 as usize);
        assert!(Path::new(&format!("{}/block-0", folder.path.clone())).exists() == false);
        assert!(Path::new(&format!("{}/block-1", folder.path.clone())).exists() == false);
        assert!(Path::new(&format!("{}/block-2", folder.path.clone())).exists() == false);
    }

    #[test]
    fn test_sharded_block_file_object_trim_case_2() {
        // Case 2:
        // Trim range contains the first shard, but partially contains the last shard results in
        // deletion of the first shard but partially write zeroes to the last shard
        let folder = TempFolder::new();
        let mut size_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(format!("{}/size", folder.path.clone()))
                            .unwrap();
        size_file.write(b"16777216");
        let mut sharded_block = ShardedBlock::new(
            String::from("test"),
            format!("file:///{}", folder.path.clone())
        );
        assert!(sharded_block.size_of_volume() == 16 * 1024 * 1024);
        sharded_block.write(0_u64, 16 * 1024 * 1024 as usize, &[1_u8; 16 * 1024 * 1024]);

        sharded_block.trim(0_u64, 12 * 1024 * 1024 - 10 as usize);
        assert!(Path::new(&format!("{}/block-0", folder.path.clone())).exists() == false);
        assert!(Path::new(&format!("{}/block-1", folder.path.clone())).exists() == false);
        assert!(Path::new(&format!("{}/block-2", folder.path.clone())).exists() == true);
        let read_result = sharded_block.read(8 * 1024 * 1024 as u64, sharded_block.shard_size as usize).unwrap();
        let mut expected_read_result = vec![0_u8; sharded_block.shard_size as usize - 10];
        expected_read_result.extend_from_slice(&vec![1_u8; 10]);
        assert!(read_result == expected_read_result);
    }

    #[test]
    fn test_sharded_block_file_object_trim_case_3() {
        // Case 3:
        // Trim range partially contains the first shard and fully contains the last shard results
        // in deletion of the last shard but partially write zeroes to the first shard
        let folder = TempFolder::new();
        let mut size_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(format!("{}/size", folder.path.clone()))
                            .unwrap();
        size_file.write(b"16777216");
        let mut sharded_block = ShardedBlock::new(
            String::from("test"),
            format!("file:///{}", folder.path.clone())
        );
        assert!(sharded_block.size_of_volume() == 16 * 1024 * 1024);
        sharded_block.write(0_u64, 16 * 1024 * 1024 as usize, &[1_u8; 16 * 1024 * 1024]);

        sharded_block.trim(10_u64, 12 * 1024 * 1024 - 10 as usize);
        assert!(Path::new(&format!("{}/block-0", folder.path.clone())).exists() == true);
        assert!(Path::new(&format!("{}/block-1", folder.path.clone())).exists() == false);
        assert!(Path::new(&format!("{}/block-2", folder.path.clone())).exists() == false);
        let read_result = sharded_block.read(0_u64, sharded_block.shard_size as usize).unwrap();
        let mut expected_read_result = vec![1_u8; 10];
        expected_read_result.extend_from_slice(&vec![0_u8; sharded_block.shard_size as usize - 10]);
        assert!(read_result == expected_read_result);
    }

    #[test]
    fn test_sharded_block_file_object_trim_case_4() {
        // Case 4:
        // Trim range only contains a intermediary part of a single shard
        let folder = TempFolder::new();
        let mut size_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(format!("{}/size", folder.path.clone()))
                            .unwrap();
        size_file.write(b"16777216");
        let mut sharded_block = ShardedBlock::new(
            String::from("test"),
            format!("file:///{}", folder.path.clone())
        );
        assert!(sharded_block.size_of_volume() == 16 * 1024 * 1024);
        sharded_block.write(0_u64, 16 * 1024 * 1024 as usize, &[1_u8; 16 * 1024 * 1024]);

        sharded_block.trim(10_u64, 4 * 1024 * 1024 - 20 as usize);
        assert!(Path::new(&format!("{}/block-0", folder.path.clone())).exists() == true);
        let read_result = sharded_block.read(0_u64, sharded_block.shard_size as usize).unwrap();
        let mut expected_read_result = vec![1_u8; 10];
        expected_read_result.extend_from_slice(&vec![0_u8; sharded_block.shard_size as usize - 20]);
        expected_read_result.extend_from_slice(&vec![1_u8; 10]);
        assert!(read_result == expected_read_result);
    }

    #[test]
    fn test_sharded_block_file_object_trim_case_5() {
        // Case 5:
        // Trim range overlaps from one shard to another, fully contains neither of them
        let folder = TempFolder::new();
        let mut size_file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(format!("{}/size", folder.path.clone()))
                            .unwrap();
        size_file.write(b"16777216");
        let mut sharded_block = ShardedBlock::new(
            String::from("test"),
            format!("file:///{}", folder.path.clone())
        );
        assert!(sharded_block.size_of_volume() == 16 * 1024 * 1024);
        sharded_block.write(0_u64, 16 * 1024 * 1024 as usize, &[1_u8; 16 * 1024 * 1024]);

        sharded_block.trim(10_u64, 4 * 1024 * 1024 as usize);
        assert!(Path::new(&format!("{}/block-0", folder.path.clone())).exists() == true);
        assert!(Path::new(&format!("{}/block-1", folder.path.clone())).exists() == true);
        let read_result = sharded_block.read(0_u64, 2 * sharded_block.shard_size as usize).unwrap();
        let mut expected_read_result = vec![1_u8; 10];
        expected_read_result.extend_from_slice(&vec![0_u8; sharded_block.shard_size as usize]);
        expected_read_result.extend_from_slice(&vec![1_u8; sharded_block.shard_size as usize - 10]);
        assert!(read_result == expected_read_result);
    }
}
