use std::io::{Error,ErrorKind};
use url::{Url};

use crate::object::ObjectStorage;
use crate::object::FileBackend;
use crate::object::S3Backend;

pub fn storage_with_config(config: String) -> Result<Box<dyn ObjectStorage>, Error> {
    let issue_list_url = Url::parse(&config)
        .expect("Failed to parse config (URL)");

    println!("Storage: {:?}", &issue_list_url);

    return match issue_list_url.scheme() {
        "file" => {
            // Expecting a folder path
            let mut path_from_url = issue_list_url.as_str().splitn(2, "///");
            path_from_url.next().unwrap(); // 'file:'
            let path = path_from_url.next().unwrap_or("./");
            Ok(Box::new(FileBackend::new(String::from(path))))
        },
        "s3+http" | "s3+https" => {
            Ok(Box::new(S3Backend::new(config.strip_prefix("s3+").unwrap().to_string())))
        },
        _ => {
            // hard fail
            Err(Error::new(ErrorKind::Unsupported, "Not Supported"))
        }
    };
}
