use std::io::{Error,ErrorKind};
use url::{Url, ParseError};

use crate::object::ObjectStorage;
use crate::object::FileBackend;

pub fn storage_with_config(config: String) -> Result<Box<dyn ObjectStorage>, Error> {
    let issue_list_url = Url::parse(&config)
        .expect("Failed to parse config (URL)");

    return match issue_list_url.scheme() {
        "file" => {
            // Expecting a folder path
            Ok(Box::new(FileBackend::new(issue_list_url.path())))
        },
        _ => {
            // hard fail
            Err(Error::new(ErrorKind::Unsupported, "Not Supported"))
        }
    };
}
