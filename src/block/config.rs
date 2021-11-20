use std::io::{Error,ErrorKind};
use url::{Url};

use crate::block::BlockStorage;
use crate::block::RawBlock;
use crate::block::ShardedBlock;

pub fn block_storage_with_config(export_name: String, driver: String, config: String) -> Result<Box<dyn BlockStorage>, Error> {
    log::info!("block storage: {:?}", driver.clone());

    match driver.as_str() {
        "raw" => {
            Ok(Box::new(RawBlock::new(export_name.clone(), config)))
        },
        "sharded" => {
            Ok(Box::new(ShardedBlock::new(export_name.to_lowercase().clone(), config)))
        },
        _ => {
            log::error!("No such storage driver: {}", driver);
            Err(Error::new(ErrorKind::Other, "Invalid storage driver"))
        }
    }
}
