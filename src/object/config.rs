use std::io::{Error,ErrorKind};

use crate::object::ObjectStorage;
use crate::object::FileBackend;
use crate::object::S3Backend;
use crate::object::CacheBackend;

pub fn object_storage_with_config(config: String) -> Result<Box<dyn ObjectStorage>, Error> {
    // config sample; "file:/path/to/folder/"
    // config sample; "s3:http://localhost:9000/test"
    // config sample; "cache:s3:http://localhost:9000/test"

    let mut split: Vec<&str> = config.split(":").collect();
    let driver_name = split.remove(0);
    let driver_config = split.join(":");

    log::info!("object storage: {}({:?})", &driver_name, &driver_config);

    return match driver_name {
        "file" => {
            Ok(Box::new(FileBackend::new(driver_config.replace("///", "./"))))
        },
        "s3" => {
            Ok(Box::new(S3Backend::new(driver_config)))
        },
        "cache" => {
            Ok(Box::new(CacheBackend::new(driver_config)))
        },
        _ => {
            // hard fail
            Err(Error::new(ErrorKind::Unsupported, "Not Supported"))
        }
    };
}
