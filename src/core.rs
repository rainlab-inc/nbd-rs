use std::error::Error;
use regex::Regex;

use crate::nbd::{NBDExport, NBDServer};



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



pub fn export_init(size_str: &str, driver_str: &str, driver_cfg_str: &str) -> Result<(), Box<dyn Error>> {
 
    let size = human_size_to_usize(size_str)?;
    let export = NBDExport::new(String::from("noname"), size, String::from(driver_str), String::from(driver_cfg_str));


    Ok(())
}
