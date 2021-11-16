use std::{
    io::{Read, Error, ErrorKind},
    time::{Duration},
};
use url::{Url};
use log;

use crate::object::{
    ObjectStorage,
    SimpleObjectStorage,
    PartialAccessObjectStorage,
    StreamingObjectStorage,
    StreamingPartialAccessObjectStorage,
};

// https://crates.io/crates/rusty-s3/0.2.0
use reqwest::blocking::Client;
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use rusty_s3::actions::{GetObject,CreateBucket};

#[derive(Debug)]
struct S3Config {
    name: String,
    region: String,
    endpoint: Url,
    credentials: Credentials,
    path_style: UrlStyle,
}

struct S3Client {
    config: S3Config
}

struct S3ObjectMeta {
    bucket: String,
    name: String,
    size: u64,
}

impl S3Client {
    pub fn new(config: S3Config) -> S3Client {
        println!("S3Client.config: {:?}", &config);
        S3Client {
            config: config
        }
    }

    pub fn ensure_bucket(&self, name: String) -> Result<(), Error> {
        let client = Client::new();
        let bucket = Bucket::new(self.config.endpoint.clone(), self.config.path_style, name, self.config.region.clone()).unwrap();
        let action = CreateBucket::new(&bucket, &self.config.credentials);
        let signed_url = action.sign(Duration::from_secs(30)); // 30 secs
        let response = client
            .put(signed_url)
            .send();

        if response.is_err() {
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }
        let response_err = response.unwrap().error_for_status();
        if response_err.is_err() {
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }
        Ok(())
    }

    pub fn get_object_list(&self, bucket: String) -> Result<Vec<String>, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    pub fn get_object(&self, bucket_name: String, name: String) -> Result<Vec<u8>, Error> {
        log::debug!("S3Client.get_object({}, {})", bucket_name.clone(), name.clone());
        let client = Client::new();
        let bucket = Bucket::new(self.config.endpoint.clone(), self.config.path_style, bucket_name, self.config.region.clone()).unwrap();
        let mut action = GetObject::new(&bucket, Some(&self.config.credentials), &name);
        action
            .query_mut()
            .insert("response-cache-control", "no-cache, no-store");
        let signed_url = action.sign(Duration::from_secs(30)); // 30 secs
        let response_res = client.get(signed_url).send();
        log::trace!("S3Client.get_object: response handling");

        if response_res.is_err() {
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }

        let response_mayerr = response_res.unwrap().error_for_status();
        if response_mayerr.is_err() {
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }

        let mut response = response_mayerr.unwrap();
        let mut data = Vec::new();
        if response.read_to_end(&mut data).is_err() {
            return Err(Error::new(ErrorKind::Other, "S3 req failed: unable to read"));
        }

        Ok(data)
    }

    pub fn get_object_partial(&self, bucket: String, name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        // Range: bytes=2651761- kindof request header is supported, apparently
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    pub fn get_object_meta(&self, bucket: String, name: String) -> Result<S3ObjectMeta, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    pub fn delete_object(&self, bucket: String, name: String) -> Result<(), Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    pub fn put_object(&self, bucket: String, name: String, data: &[u8]) -> Result<S3ObjectMeta, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

}

pub struct S3Backend {
    url: String,
    client: S3Client,
    bucket: String,
}

impl S3Backend {
    pub fn new(url: String) -> S3Backend {
        let parsed_url = Url::parse(&url)
            .expect("Failed to parse config (URL)");

        S3Backend {
            url: url.clone(),
            client: S3Client::new(S3Config {
                name: "nbd-rs".to_string(),
                region: "eu-west-1".to_string(), // TODO: Derive from URL
                endpoint: format!("{}://{}", parsed_url.scheme(), parsed_url.host_str().unwrap()).parse().unwrap(),
                credentials: Credentials::new(parsed_url.username(), parsed_url.password().unwrap()),
                path_style: UrlStyle::Path, // UrlStyle::VirtualHost, // TODO: Derive from URL
            }),
            bucket: parsed_url.path_segments().unwrap().next().unwrap().to_string(),
        }
    }
}

impl SimpleObjectStorage for S3Backend {
    fn init(&mut self, conn_str: String) {
        // .. noop
    }

    fn exists(&self, object_name: String) -> Result<bool, Error> {
        let object_meta = self.client.get_object_meta(self.bucket.clone(), object_name.clone())?;
        Ok(true)
    }

    fn read(&self, object_name: String) -> Result<Vec<u8>, Error> {
        self.client.get_object(self.bucket.clone(), object_name.clone())
    }

    fn write(&self, object_name: String, data: &[u8]) -> Result<(), Error> {
        self.client.put_object(self.bucket.clone(), object_name.clone(), data)?;
        Ok(())
    }

    fn delete(&self, object_name: String) -> Result<(), Error> {
        self.client.delete_object(self.bucket.clone(), object_name.clone())
    }

    fn get_size(&self, object_name: String) -> Result<u64, Error> {
        let object_meta = self.client.get_object_meta(self.bucket.clone(), object_name.clone())?;
        Ok(object_meta.size)
    }

    fn start_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // NOOP
        Ok(())
    }

    fn end_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // NOOP
        Ok(())
    }

    fn persist_object(&self, object_name: String) -> Result<(), Error> {
        // NOOP
        Ok(())
    }
}

impl PartialAccessObjectStorage for S3Backend {

    fn partial_read(&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        self.client.get_object_partial(self.bucket.clone(), object_name.clone(), offset, length)
    }

    fn partial_write(&self, object_name: String, offset: u64, length: usize, data: &[u8]) -> Result<usize, Error> {
        // isn't supported by S3 (easily..), so need manual patching of the object, race-prune
        // TODO: FETCH into Vec<u8>
        // TODO: PATCH it partially
        // TODO: PUT BACK
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }
}

impl StreamingObjectStorage for S3Backend {}
impl StreamingPartialAccessObjectStorage for S3Backend {}
impl ObjectStorage for S3Backend {}
