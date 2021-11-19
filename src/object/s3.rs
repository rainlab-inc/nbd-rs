use std::{
    io::{Read, Error, ErrorKind},
    time::{Duration, Instant},
    collections::{HashMap},
    rc::{Rc},
    cell::{RefCell},
};
use url::{Url};

use log;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;

use crate::object::{
    ObjectStorage,
    SimpleObjectStorage,
    PartialAccessObjectStorage,
    StreamingObjectStorage,
    StreamingPartialAccessObjectStorage,
};
use crate::util::Propagation;

#[derive(Debug)]
struct S3Config {
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
        log::debug!("S3Client.config: {:?}", &config);
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

    pub fn delete_object(&self, bucket_name: String, name: String) -> Result<(), Error> {
        log::debug!("S3Client.delete_object({}, {})", bucket_name.clone(), name.clone());
        let bucket = self.bucket(bucket_name.clone());
        let object_res = bucket.delete_object_blocking(name.clone());

        if object_res.is_err() {
            log::error!("S3 Error: {}", object_res.err().unwrap());
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }

        let (_, status) = object_res.unwrap();
        if status == 404 {
            log::debug!("S3Client.delete_object: NotFound");
            return Err(Error::new(ErrorKind::NotFound, "Object Not Found"));
        }
        else if status != 204 {
            return Err(Error::new(ErrorKind::Other, format!("S3 req failed: HTTP Status {}", status)));
        }

        Ok(())
    }

    pub fn put_object(&self, bucket_name: String, name: String, data: &[u8]) -> Result<S3ObjectMeta, Error> {
        log::debug!("S3Client.put_object({}, {})", bucket_name.clone(), name.clone());
        let bucket = self.bucket(bucket_name.clone());
        let size = data.len();
        let object_res = bucket.put_object_blocking(name.clone(), data);

        if object_res.is_err() {
            log::error!("S3 Error: {}", object_res.err().unwrap());
            return Err(Error::new(ErrorKind::Other, "S3 req failed"));
        }

        let (data, status) = object_res.unwrap();
        if status == 404 {
            return Err(Error::new(ErrorKind::NotFound, "Bucket Not Found"));
        }
        else if status != 200 && status != 201 {
            let body = String::from_utf8_lossy(&data);
            log::error!("HTTP({}): {}", status, body);
            return Err(Error::new(ErrorKind::Other, format!("S3 req failed: HTTP Status {}", status)));
        }

        let object_meta = S3ObjectMeta {
            bucket: bucket_name.clone(),
            name: name.clone(),
            size: size as u64,
        };

        Ok(object_meta)
    }

}

pub struct S3CachedObject {
    data: Vec<u8>,
    size: usize,
    keep: u16,
    reads: u16,
    writes: u16,
    persists: u16,
    last_read: Option<Instant>,
    last_write: Option<Instant>,
    last_persist: Option<Instant>,
}

pub struct S3Backend {
    url: String,
    client: S3Client,
    bucket: String,
    cache: RefCell<HashMap<String, Rc<RefCell<S3CachedObject>>>>,
    mem_usage: RefCell<usize>,
}

impl S3Backend {
    pub fn new(url: String) -> S3Backend {
        let parsed_url = Url::parse(&url)
            .expect("Failed to parse config (URL)");

        let password = parsed_url.password().unwrap();
        S3Backend {
            url: url.clone(),
            client: S3Client::new(S3Config {
                region: "minio".to_string(), // TODO: Derive from URL
                endpoint: format!("{}://{}:{}",
                    parsed_url.scheme(),
                    parsed_url.host_str().unwrap(),
                    parsed_url.port_or_known_default().unwrap().to_string(),
                    ).to_string(),
                access_key: parsed_url.username().clone().to_string(),
                secret_key: password.clone().to_string(),
                path_style: true, // TODO: Derive from URL
            }),
            bucket: parsed_url.path_segments().unwrap().next().unwrap().to_string(),
            cache: RefCell::<HashMap<String, Rc<RefCell<S3CachedObject>>>>::new(
                HashMap::<String, Rc<RefCell<S3CachedObject>>>::new()
            ),
            mem_usage: RefCell::<usize>::new(0),
        }
    }
}

impl SimpleObjectStorage for S3Backend {
    fn init(&mut self, conn_str: String) {
        // .. noop
    }

    fn exists(&self, object_name: String) -> Result<bool, Error> {
        let cache = self.cache.borrow();
        if cache.contains_key(&object_name.clone()) {
            return Ok(true);
        }

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
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();
            cached_obj.reads += 1;
            cached_obj.last_read = Some(Instant::now());
            return Ok(cached_obj.data.clone());
        }

        let data = self.client.get_object(self.bucket.clone(), object_name.clone())?;
        let cached_object = S3CachedObject {
            data: data.clone(),
            size: data.len(),
            keep: 0,
            reads: 0,
            writes: 0,
            persists: 0,
            last_read: Some(Instant::now()),
            last_write: None,
            last_persist: None,
        };
        let mut mem_usage = self.mem_usage.borrow_mut();
        *mem_usage += cached_object.size;
        log::trace!("s3: mem_usage: {}", mem_usage);
        cache.insert(object_name.clone(), Rc::new(RefCell::new(cached_object)));
        Ok(data)
    }

    fn write(&self, object_name: String, data: &[u8]) -> Result<Propagation, Error> {
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();
            cached_obj.writes += 1;
            cached_obj.last_write = Some(Instant::now());
            cached_obj.data = data.to_vec();
            return Ok(Propagation::Queued);
        }

        let cached_object = S3CachedObject {
            data: data.to_vec(),
            size: data.len(),
            keep: 0,
            reads: 0,
            writes: 1,
            persists: 0,
            last_read: None,
            last_write: Some(Instant::now()),
            last_persist: None,
        };
        let mut mem_usage = self.mem_usage.borrow_mut();
        *mem_usage += cached_object.size;
        log::trace!("s3: mem_usage: {}", mem_usage);
        cache.insert(object_name.clone(), Rc::new(RefCell::new(cached_object)));

        // self.client.put_object(self.bucket.clone(), object_name.clone(), data)?;
        Ok(Propagation::Queued)
    }

    fn delete(&self, object_name: String) -> Result<Propagation, Error> {
        self.client.delete_object(self.bucket.clone(), object_name.clone())?;
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let cached_obj = cached_obj_ref.unwrap().1.borrow();
            let mut mem_usage = self.mem_usage.borrow_mut();
            *mem_usage -= cached_obj.size;
            log::trace!("s3: mem_usage: {}", mem_usage);
        }
        cache.remove(&object_name.clone());

        self.client.delete_object(self.bucket.clone(), object_name.clone())?;
        Ok(Propagation::Complete)
    }

    fn get_size(&self, object_name: String) -> Result<u64, Error> {
        let cache = self.cache.borrow();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let cached_obj = cached_obj_ref.unwrap().1.borrow();
            return Ok(cached_obj.size as u64);
        }

        let object_meta = self.client.get_object_meta(self.bucket.clone(), object_name.clone())?;
        Ok(object_meta.size)
    }

    fn start_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // increase
        let mut cache = self.cache.borrow_mut();
        if !cache.contains_key(&object_name.clone()) {
            // for side effect;
            self.read(object_name.clone())?;
        }

        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();
            cached_obj.keep += 1;
        }

        Ok(())
    }

    fn end_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // decrease
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();
            cached_obj.keep -= 1;
        }

        Ok(())
    }

    fn persist_object(&self, object_name: String) -> Result<Propagation, Error> {
        // NOOP
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();

            // already persisted
            if cached_obj.persists == cached_obj.writes {
                return Ok(Propagation::Redundant);
            }

            self.client.put_object(self.bucket.clone(), object_name.clone(), &cached_obj.data.clone())?;
            cached_obj.persists = cached_obj.writes;
            cached_obj.last_persist = Some(Instant::now());
            return Ok(Propagation::Complete);
        }

        Ok(Propagation::Noop)
    }
}

impl PartialAccessObjectStorage for S3Backend {

    fn partial_read(&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        // self.client.get_object_partial(self.bucket.clone(), object_name.clone(), offset, length)
        let old_buffer: Vec<u8> = self.read(object_name.clone())?;
        let slice: Vec<u8> = old_buffer[(offset as usize)..((offset as usize) + length)].to_vec();
        Ok(slice)
    }

    fn partial_write(&self, object_name: String, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error> {
        // isn't supported by S3 (easily..), so need manual patching of the object, race-prune
        // FETCH into Vec<u8>
        let old_buffer: Vec<u8> = self.read(object_name.clone())?;
        let mut new_buffer: Vec<u8> = Vec::new();

        if offset > 0 {
            new_buffer.extend_from_slice(&old_buffer[0..(offset as usize)]);
        }
        // PATCH it partially
        new_buffer.extend_from_slice(data);

        let remaining = old_buffer.len() - (offset as usize) - length;
        if remaining > 0 {
            new_buffer.extend_from_slice(&old_buffer[((offset as usize)+length)..((offset as usize)+length+remaining)]);
        }

        // PUT BACK
        self.write(object_name, &new_buffer)
    }
}

impl StreamingObjectStorage for S3Backend {}
impl StreamingPartialAccessObjectStorage for S3Backend {}
impl ObjectStorage for S3Backend {}
