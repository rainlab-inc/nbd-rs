use std::{
    str,
    io::{Error, ErrorKind},
};

use log;

use crate::{
    object::{ObjectStorage, object_storage_with_config, object_storages_with_config},
    block::{BlockStorage, BlockStorageConfig, ShardDistribution},
};
use crate::util::Propagation;

// Driver: DistributedBlock

pub struct DistributedBlock{
    name: Option<String>,
    volume_size: u64,
    shard_size: u64,
    object_storages: Vec<Box<dyn ObjectStorage>>,
    shard_distribution: ShardDistribution,
    config: BlockStorageConfig,
}

fn get_cfg_entry(split: &Vec<&str>, entry: &str) -> Option<String> {
    let entry_str = format!("{}{}",entry, "=");
    for cfg in split {
        let tmp = cfg.strip_prefix(&entry_str);
        if tmp.is_some() {
            return Some(String::from(tmp.unwrap()));
        }
    }
    None
}

impl DistributedBlock {
    pub fn new(config: BlockStorageConfig) -> DistributedBlock {
             // TODO: Allow configuring disk size in config string
        //       or a setting like `create=true`
        // TODO: Allow configuring shard size in config string
        let default_shard_size: u64 = 4 * 1024 * 1024;

        let conn_str = config.conn_str.clone();
        let split = conn_str.split(";").collect();
        let replicas: u8 = get_cfg_entry(&split, "replicas").unwrap().parse().unwrap();
        let backends = get_cfg_entry(&split, "backends").unwrap();

        let object_storages = object_storages_with_config(backends).unwrap();
        let shard_distribution = ShardDistribution::new(object_storages.len() as u8, replicas);

        let mut distributed_block = DistributedBlock {
            name: config.export_name.clone(),
            volume_size: 0,
            shard_size: default_shard_size,
            object_storages,
            shard_distribution,
            config: config.clone(),
        };

        distributed_block.init(config.init_volume).unwrap();
        distributed_block
    }

    pub fn shard_index(&self, offset: u64) -> usize {
        (offset / &self.shard_size) as usize
    }

    pub fn object_storage_index(&self, offset: u64) -> usize {
        self.shard_index(offset) % self.object_storages.len()
    }

    pub fn get_object_storage(&self, shard_idx: usize, replica_idx: u8) -> &Box<dyn ObjectStorage> {
        &self.object_storages[self.shard_distribution.node_idx_for_shard(shard_idx, replica_idx) as usize]
    }

    pub fn size_of_volume(&self) -> u64 {
        let object_name = String::from("size");
        let filedata = self.object_storages[0].read(object_name); // TODO: Errors?
        if filedata.is_err() {
            return 4 * 1024 * 1024 * 1024; // 4 GiB
        }
        // TODO: Allow file to not exist, create if does not exist
        let mut string = str::from_utf8(&filedata.unwrap()).unwrap().to_string();
        string.retain(|c| !c.is_whitespace());
        let volume_size: u64 = string.parse().unwrap();
        volume_size
    }

    pub fn shard_name(&self, shard_idx: usize, replica_idx: u8) -> String {
        format!("block-{}-{}", shard_idx, replica_idx).to_string()
    }

    pub fn get_replica_idx_from_shard(&self, shard_idx: usize) ->Result<Option<u8>, Error> {
        for replica_idx in 0..self.shard_distribution.replicas {
            let shard_name = self.shard_name(shard_idx, replica_idx);
            if self.get_object_storage(shard_idx, replica_idx).exists(shard_name.clone())? {
                return Ok(Some(replica_idx))
            }
        }
        Ok(None)
    }

}

impl BlockStorage for DistributedBlock {
    fn init(&mut self, init_volume: bool) -> Result<(), Box<dyn std::error::Error>> {
        if init_volume {
            self.init_volume()
        } else {
            self.check_volume()
        }
    }

    fn init_volume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize volume
        let volume_size = self.config.export_size.unwrap() as u64;
        log::info!("Volume size: {}", volume_size);
        self.volume_size = volume_size;
            
        /* Check initialized */
        for (i, storage) in self.object_storages.iter().enumerate() {
            let size = storage.read("size".to_string());
            if size.is_err() {
                continue;
            } else {
                let size = String::from_utf8(storage.read("size".to_string()).unwrap()).unwrap();
                let size: u64 = size.parse().unwrap();
                if size == volume_size {
                    log::warn!("Node {} is already initialized with the same size: {}", i, size);
                } else {
                    if !self.config.export_force {
                        log::error!("Node {} is already initialized and the size is configured to be {}, add --force to override current configuration", i, size);
                        panic!();
                    } else {
                        log::warn!("Node {} is already initialized with size: {}", i, size);
                    }
                }
            }
        }
        
        log::info!("Initializing volume with size: {}", volume_size);

        for (i, storage) in self.object_storages.iter().enumerate() {
            let size_str = volume_size.to_string();
            storage.write(String::from("size"), &size_str.as_bytes());
            storage.persist_object(String::from("size"));
            log::info!("Volume size written to: node-{}", i);
        }

        Ok(())
    }
    
    fn check_volume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut volume_size: u64 = 0;
        let mut first_node = true;

        for (i, storage) in self.object_storages.iter().enumerate() {
            let size = String::from_utf8(storage.read("size".to_string()).unwrap()).unwrap();
            let tmp_volume_size = size.parse().unwrap();
            log::info!("Volume size in the node-{} is {}", i, tmp_volume_size);

            if first_node {
                volume_size = tmp_volume_size;
                first_node = false;
                continue;
            }

            if tmp_volume_size != volume_size {
                return Err(Error::new(ErrorKind::Other, format!("Volume sizes should be same for each node.")).into());
            }
        }

        log::info!("Volume sizes are same for all nodes: {}", volume_size);
        self.volume_size = volume_size;
        Ok(())
    }

    fn destroy_volume(&mut self) {
        for storage in &self.object_storages {
            storage.destroy();
        }
        log::info!("The volume is destroyed.");
    }
    
    fn get_name(&self) -> String {
        self.name.clone().unwrap()
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

            let replica_idx = self.get_replica_idx_from_shard(i)?;
            
            if replica_idx.is_some(){
                let replica_idx = replica_idx.unwrap();
                let shard_name = self.shard_name(i, replica_idx);
                log::trace!("storage::read(iteration: {} replica: {})", i, replica_idx);

                if i == start {
                    let read_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                    let buf = self.get_object_storage(i, replica_idx)
                        .partial_read(shard_name.clone(), offset % self.shard_size, read_size)?;
                    buffer.extend_from_slice(&buf);
                    continue;
                }
                if i == end {
                    let mut read_size = ((length as u64 + offset) % self.shard_size) as usize;
                    if read_size == 0 {
                        read_size = self.shard_size as usize;
                    }
                    let buf = self.get_object_storage(i, replica_idx)
                        .partial_read(shard_name.clone(), 0, read_size)?;
                    buffer.extend_from_slice(&buf);
                    break;
                }
                let buf = self.get_object_storage(i, 0)
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
        // FIXME! 
        // This is so wrong.
        // We are trying to write same data more than once but write function returns propagation which depends only
        // 1 node so for temporary, propagation of the first node is returned.
        let mut overall_first_propagation = Propagation::Noop;
        for replica_idx in 0..self.shard_distribution.replicas {
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
                let shard_name = self.shard_name(cur_shard, replica_idx);

                let slice = &data[written..(written + write_len)];
                let propagated;

                // full write
                if write_len == self.shard_size as usize {
                    propagated = self.get_object_storage(cur_shard, replica_idx).write(shard_name.clone(), slice)?;
                }
                // new object
                else if !self.get_object_storage(cur_shard, replica_idx).exists(shard_name.clone())? {
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
                    propagated = self.get_object_storage(cur_shard, replica_idx).write(shard_name.clone(), &buffer)?;

                    // existing object, partial write
                } else {
                    propagated = self.get_object_storage(cur_shard, replica_idx).partial_write(shard_name.clone(), shard_offset as u64, write_len, slice)?;
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
 
            if replica_idx == 0 {
                overall_first_propagation = overall_propagation;
            }
        }
        Ok(overall_first_propagation)
    }

    fn flush(&mut self, offset: u64, length: usize) -> Result<Propagation, Error> {
        // FIXME! 
        // This is so wrong.
        // We are trying to flush same data more than once but flush function returns propagation which depends only
        // 1 node so for temporary, propagation of the first node is returned.
        let mut overall_first_propagation = Propagation::Noop;
        for replica_idx in 0..self.shard_distribution.replicas {
            let start = self.shard_index(offset);
            let end = if 0 == (offset + length as u64) % self.shard_size {
                self.shard_index(offset + length as u64) - 1
            } else {
                self.shard_index(offset + length as u64)
            };

            log::debug!("storage::flush(start: {}, end: {})", start, end);
            let mut overall_propagation : Propagation = Propagation::Guaranteed;
            for i in start..=end {
                let shard_name = self.shard_name(i, replica_idx);
                let propagated = self.get_object_storage(i, replica_idx).persist_object(shard_name.clone())?;
                if (propagated as u8) >= (Propagation::Queued as u8) {
                    log::debug!("storage::flush(iteration: {}, {})", i, propagated as u8);
                } else {
                    log::trace!("storage::flush(iteration: {}, {})", i, propagated as u8);
                }
                if (propagated as u8) < (overall_propagation as u8) {
                    overall_propagation = propagated;
                }
            }
            if replica_idx == 0 {
                overall_first_propagation = overall_propagation;
            }
        }
        Ok(overall_first_propagation)
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
            let object_name = self.shard_name(i, 0);
            if i == start {
                let trim_size = std::cmp::min((self.shard_size - offset % self.shard_size) as usize, length);
                if trim_size as u64 % self.shard_size == 0 {
                    overall_propagation = self.get_object_storage(i, 0).delete(object_name)?;
                } else {
                    overall_propagation = self.get_object_storage(i, 0).partial_write(
                        object_name,
                        offset % self.shard_size,
                        trim_size,
                        &vec![0_u8; trim_size]
                    )?;
                }
            } else if i == end {
                let trim_size = ((length as u64 + offset % self.shard_size) % self.shard_size) as usize;
                if trim_size as u64 % self.shard_size == 0 {
                    overall_propagation = self.get_object_storage(i, 0).delete(object_name)?;
                } else {
                    overall_propagation = self.get_object_storage(i, 0).partial_write(
                        object_name,
                        0,
                        trim_size,
                        &vec![0_u8; trim_size]
                    )?;
                }
            } else {
                overall_propagation = self.get_object_storage(i, 0).delete(object_name)?;
            }
        }
        Ok(overall_propagation)
    }

    fn close(&mut self) {
        log::debug!("storage::close");
        for object_storage in self.object_storages.iter_mut(){
            object_storage.close();
        }
    }
}
