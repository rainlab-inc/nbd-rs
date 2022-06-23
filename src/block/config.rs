use std::io::{Error,ErrorKind};

use crate::block::BlockStorage;
use crate::block::RawBlock;
use crate::block::ShardedBlock;
use crate::block::DistributedBlock;

pub struct BlockStorageConfig {
    pub export_name: Option<String>,
    pub export_size: Option<usize>,
    pub export_force: bool,
    pub driver: String,
    pub conn_str: String,
}

pub fn block_storage_with_config(config: BlockStorageConfig) -> Result<Box<dyn BlockStorage>, Error> {
    log::info!("block storage: {:?}", config.driver.clone());

    match config.driver.as_str() {
        "raw" => {
            Ok(Box::new(RawBlock::new(config)))
        },
        "sharded" => {
            Ok(Box::new(ShardedBlock::new(config)))
        },
        "distributed" => {
            Ok(Box::new(DistributedBlock::new(config)))
        }
        _ => {
            log::error!("No such storage driver: {}", config.driver);
            Err(Error::new(ErrorKind::Other, "Invalid storage driver"))
        }
    }
}
