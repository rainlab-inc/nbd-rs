use std::error::Error;
use regex::Regex;

use crate::nbd::{NBDExport, NBDServer};
use crate::block::{BlockStorageConfig, block_storage_with_config};
use std::sync::{Arc, RwLock};


fn human_size_to_usize(size_str: &str) -> Result<usize, Box<dyn Error>> {
    let kb = 10_usize.pow(3);
    let mb = 10_usize.pow(6);
    let m = mb;
    let gb = 10_usize.pow(9);
    let g = gb;

    let re = Regex::new(r"(\d*)(kB|MB|M|GB|G)\b")?;
    for cap in re.captures(size_str) {
        let size: usize = cap[1].parse()?;
        let multipler = match &cap[2] {
            "kB" => kb,
            "MB" => mb,
            "M" =>  m,
            "GB" => gb,
            "G" =>  g,
            _  => return Err("unreachable".into()),
        };

        return Ok(size * multipler)
    }
    return Err("unreachable".into());
}



pub fn export_init(size_str: &str, driver_str: &str, driver_cfg_str: &str, force: bool) -> Result<(), Box<dyn Error>> {

    let size = human_size_to_usize(size_str)?;

    let config = BlockStorageConfig {
        export_name: None,
        export_size: Some(size),
        export_force: force,
        driver: driver_str.to_string(),
        conn_str: driver_cfg_str.to_string(),
    };

    let _block_storage = block_storage_with_config(config)?;
    Ok(())
}

pub fn export_serve(export_name: &str, driver_str: &str, driver_cfg_str: &str) -> Result<(), Box<dyn Error>> {
    let mut exports = Vec::<Arc<RwLock<NBDExport>>>::new();

    let export = Arc::new(RwLock::new(NBDExport::new(
                export_name.to_string(),
                String::from(driver_str),
                String::from(driver_cfg_str),
                )));

    exports.push(export);

    let mut server = NBDServer::new("0.0.0.0".to_string(), 10809, exports);
    server.listen();
    Ok(())
}
