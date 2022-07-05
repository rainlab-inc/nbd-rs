use std::error::Error;

use crate::nbd::{NBDExport, NBDServer};
use crate::block::{BlockStorageConfig, block_storage_with_config};
use crate::util::{human_size_to_usize};
use std::sync::{Arc, RwLock};

pub fn init_export(size_str: &str, driver_str: &str, driver_cfg_str: &str, force: bool) -> Result<(), Box<dyn Error>> {
    let size = human_size_to_usize(size_str)?;

    let config = BlockStorageConfig {
        export_name: None,
        export_size: Some(size),
        export_force: force,
        driver: driver_str.to_string(),
        conn_str: driver_cfg_str.to_string(),
        init_volume: true,
    };

    block_storage_with_config(config)?;
    Ok(())
}

pub fn serve_exports(exports: Vec::<Arc<RwLock<NBDExport>>>) -> Result<(), Box<dyn Error>> {
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10809, exports);
    server.listen();
    Ok(())
}

pub fn destroy_export(driver_str: &str, driver_cfg_str: &str) -> Result<(), Box<dyn Error>> {
    let config = BlockStorageConfig {
        export_name: None,
        export_size: None,
        export_force: true,
        driver: driver_str.to_string(),
        conn_str: driver_cfg_str.to_string(),
        init_volume: false,
    };

    let mut block_storage = block_storage_with_config(config)?;
    block_storage.destroy_volume();

    Ok(())

}
