use std::{
    io::{Read, Error, ErrorKind},
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

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;

#[derive(Debug)]
struct S3Config {
    name: String,
    region: String,
    endpoint: String,
    access_key: String,
    secret_key: String,
    path_style: bool,
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

    pub fn bucket(&self, name: String) -> Bucket {
        let credentials = Credentials::new(Some(&self.config.access_key.clone()), Some(&self.config.secret_key.clone()), None, None, None).unwrap();
        let region = Region::Custom { region: self.config.region.clone(), endpoint: self.config.endpoint.clone() };
        let mut bucket = Bucket::new(&name, region, credentials).unwrap();
        if self.config.path_style {
            bucket.set_path_style();
        }
        bucket
    }

    pub fn ensure_bucket(&self, name: String) -> Result<(), Error> {
        //let bucket = self.bucket(name);
        //bucket.
        Ok(())
    }

    pub fn get_object_list(&self, bucket: String) -> Result<Vec<String>, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented"))
    }

    pub fn get_object(&self, bucket_name: String, name: String) -> Result<Vec<u8>, Error> {
        log::debug!("S3Client.get_object({}, {})", bucket_name.clone(), name.clone());
        let bucket = self.bucket(bucket_name);
        let object_res = bucket.get_object_blocking(name);

        if object_res.is_err() {
            log::error!("S3 Error: {}", object_res.err().unwrap());
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }

        let (data, status) = object_res.unwrap();
        if status == 404 {
            return Err(Error::new(ErrorKind::NotFound, "Object Not Found"));
        }
        else if status != 200 {
            let body = String::from_utf8_lossy(&data);
            log::error!("HTTP({}): {}", status, body);
            return Err(Error::new(ErrorKind::Other, format!("S3 req failed: HTTP Status {}", status)));
        }

        Ok(data)
    }

    pub fn get_object_partial(&self, bucket: String, name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        // Range: bytes=2651761- kindof request header is supported, apparently
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented: S3Client::get_object_partial"))
    }

    pub fn get_object_meta(&self, bucket_name: String, name: String) -> Result<S3ObjectMeta, Error> {
        log::debug!("S3Client.get_object_meta({}, {})", bucket_name.clone(), name.clone());
        let bucket = self.bucket(bucket_name.clone());
        let object_res = bucket.head_object_blocking(name.clone());

        if object_res.is_err() {
            log::error!("S3 Error: {}", object_res.err().unwrap());
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }

        let (data, status) = object_res.unwrap();
        if status == 404 {
            log::debug!("S3Client.get_object_meta: NotFound");
            return Err(Error::new(ErrorKind::NotFound, "Object Not Found"));
        }
        else if status != 200 {
            return Err(Error::new(ErrorKind::Other, format!("S3 req failed: HTTP Status {}", status)));
        }

        let object_meta = S3ObjectMeta {
            bucket: bucket_name.clone(),
            name: name.clone(),
            size: data.content_length.unwrap_or(0) as u64,
        };

        Ok(object_meta)
    }

    pub fn delete_object(&self, bucket: String, name: String) -> Result<(), Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented: S3Client::delete_object"))
    }

    pub fn put_object(&self, bucket: String, name: String, data: &[u8]) -> Result<S3ObjectMeta, Error> {
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented: S3Client::put_object"))
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

        let password = parsed_url.password().unwrap();
        S3Backend {
            url: url.clone(),
            client: S3Client::new(S3Config {
                name: "nbd-rs".to_string(),
                region: "eu-west-1".to_string(), // TODO: Derive from URL
                endpoint: format!("{}://{}:{}",
                    parsed_url.scheme(),
                    parsed_url.host_str().unwrap(),
                    parsed_url.port_or_known_default().unwrap().to_string(),
                    ).to_string(),
                access_key: parsed_url.username().clone().to_string(),
                secret_key: password.clone().to_string(),
                path_style: true,
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
        let object_meta = self.client.get_object_meta(self.bucket.clone(), object_name.clone());
        if object_meta.is_err() {
            let err = object_meta.err().unwrap();
            if err.kind() == ErrorKind::NotFound {
                return Ok(false);
            }
            return Err(err);
        }
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
        Err(Error::new(ErrorKind::Unsupported, "Not yet implemented: S3Backend::partial_write"))
    }
}

impl StreamingObjectStorage for S3Backend {}
impl StreamingPartialAccessObjectStorage for S3Backend {}
impl ObjectStorage for S3Backend {}
