use std::{
    io::{Error, ErrorKind},
    time::{Instant},
    collections::{HashMap},
    rc::{Rc},
    cell::{RefCell},
};
use url::{Url};

use log;

use crate::object::{
    object_storage_with_config,
    ObjectStorage,
    SimpleObjectStorage,
    PartialAccessObjectStorage,
    StreamingObjectStorage,
    StreamingPartialAccessObjectStorage,
};
use crate::util::Propagation;

pub struct CachedObject {
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

pub struct CacheBackend {
    config: String,
    backend: Box<dyn ObjectStorage>,
    cache: RefCell<HashMap<String, Rc<RefCell<CachedObject>>>>,
    mem_usage: RefCell<usize>,
    mem_limit: usize,
    max_stall_secs: u16,
}

impl CacheBackend {
    pub fn new(config: String) -> CacheBackend {
        let mut split: Vec<&str> = config.split(",").collect();
        let backend_url = split.pop().unwrap();
        let parsed_url = Url::parse(&backend_url)
            .expect("Failed to parse backend (URL)");

        // TODO: Parse remaining parts from split for configuring;
        //   mem_limit = 64M
        //   max_stall = 15s

        CacheBackend {
            config: config.clone(),
            backend: object_storage_with_config(config).unwrap(),
            cache: RefCell::<HashMap<String, Rc<RefCell<CachedObject>>>>::new(
                HashMap::<String, Rc<RefCell<CachedObject>>>::new()
            ),
            mem_usage: RefCell::<usize>::new(0),
            mem_limit: 64 * 1024 * 1024,
            max_stall_secs: 15, // persist things to disk after 15sec max
        }
    }
}

impl SimpleObjectStorage for CacheBackend {
    fn init(&mut self, conn_str: String) {
        // .. noop
        log::info!("init");
    }

    fn exists(&self, object_name: String) -> Result<bool, Error> {
        let cache = self.cache.borrow();
        if cache.contains_key(&object_name.clone()) {
            log::trace!("exists: hit");
            return Ok(true);
        }

        log::trace!("exists: miss");
        // TODO: Cache exists|not status as well?
        self.backend.exists(object_name)
    }

    fn read(&self, object_name: String) -> Result<Vec<u8>, Error> {
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("read: hit");
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();
            cached_obj.reads += 1;
            cached_obj.last_read = Some(Instant::now());
            return Ok(cached_obj.data.clone());
        }

        log::trace!("read: miss");
        let data = self.backend.read(object_name.clone())?;
        let cached_object = CachedObject {
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
        log::debug!("mem_usage: {}", mem_usage);
        cache.insert(object_name.clone(), Rc::new(RefCell::new(cached_object)));
        Ok(data)
    }

    fn write(&self, object_name: String, data: &[u8]) -> Result<Propagation, Error> {
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("write: hit");
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();
            cached_obj.writes += 1;
            cached_obj.last_write = Some(Instant::now());
            cached_obj.data = data.to_vec();
            log::debug!("mem_usage: {}", self.mem_usage.borrow());
            return Ok(Propagation::Queued);
        }

        log::trace!("write: miss");
        let cached_object = CachedObject {
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
        // TODO: Check if mem limit allows this, otherwise
        // * block until it allows, and pressure cache purge
        *mem_usage += cached_object.size;
        log::debug!("mem_usage: {}", mem_usage);
        cache.insert(object_name.clone(), Rc::new(RefCell::new(cached_object)));

        Ok(Propagation::Queued)
    }

    fn delete(&self, object_name: String) -> Result<Propagation, Error> {
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let cached_obj = cached_obj_ref.unwrap().1.borrow();
            let mut mem_usage = self.mem_usage.borrow_mut();
            *mem_usage -= cached_obj.size;
            log::debug!("mem_usage: {}", mem_usage);
        }
        cache.remove(&object_name.clone());

        self.backend.delete(object_name.clone())
    }

    fn get_size(&self, object_name: String) -> Result<u64, Error> {
        let cache = self.cache.borrow();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("size: hit");
            let cached_obj = cached_obj_ref.unwrap().1.borrow();
            return Ok(cached_obj.size as u64);
        }

        log::trace!("size: miss");
        // TODO: cache this(size only) as well??
        self.backend.get_size(object_name.clone())
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

        self.backend.start_operations_on_object(object_name.clone())
    }

    fn end_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // decrease
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();
            cached_obj.keep -= 1;
        }

        self.backend.end_operations_on_object(object_name.clone())
    }

    fn persist_object(&self, object_name: String) -> Result<Propagation, Error> {
        let mut cache = self.cache.borrow_mut();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.borrow_mut();

            // already persisted
            if cached_obj.persists == cached_obj.writes {
                return Ok(Propagation::Redundant);
            }

            log::debug!("persist: hit");
            let write_propagation = self.backend.write(object_name.clone(), &cached_obj.data.clone())?;
            cached_obj.persists = cached_obj.writes;
            cached_obj.last_persist = Some(Instant::now());

            let persist_propagation = self.backend.persist_object(object_name.clone())?;
            if (persist_propagation as u8) > (write_propagation as u8) {
                return Ok(persist_propagation);
            }

            return Ok(write_propagation);
        }

        self.backend.persist_object(object_name.clone())
    }
}

impl PartialAccessObjectStorage for CacheBackend {

    fn partial_read(&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        let cache = self.cache.borrow();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("partial_read: hit");
            let cached_obj = cached_obj_ref.unwrap().1.borrow();
            let data: Vec<u8> = cached_obj.data.clone();
            let slice: Vec<u8> = data[(offset as usize)..((offset as usize) + length)].to_vec();
            return Ok(slice);
        }
        drop(cache);

        // // not cached; try backend
        log::trace!("partial_read: miss");
        // let backend_read_res = self.backend.partial_read(object_name.clone(), offset, length);
        // if backend_read_res.is_ok() {
        //     return Ok(backend_read_res.unwrap());
        // }

        // let err = backend_read_res.err().unwrap();
        // if err.kind() == ErrorKind::Unsupported {
            log::trace!("partial_read: emulate");
            let old_buffer: Vec<u8> = self.read(object_name.clone())?;
            let slice: Vec<u8> = old_buffer[(offset as usize)..((offset as usize) + length)].to_vec();
            return Ok(slice);
        // }

        // if err.kind() == ErrorKind::NotFound {
        //     // TODO: Cache NotFound state?
        // }

        // Err(err)
    }

    fn partial_write(&self, object_name: String, offset: u64, length: usize, data: &[u8]) -> Result<Propagation, Error> {
        let cache = self.cache.borrow();
        if !cache.contains_key(&object_name.clone()) {
            // code below commented out, because if we proxy this to backend,
            // and if the backend has an ugly workaround for this,
            // the client will be repeatedly using the ugly backend for this,
            // when the cache would have been much better..

            // log::trace!("partial_write: miss -> proxy");
            // // try proxy
            // let backend_write_res = self.backend.partial_write(object_name.clone(), offset, length, data);
            // if backend_write_res.is_ok() {
            //     return backend_write_res;
            // }

            // let err = backend_write_res.err().unwrap();
            // if err.kind() != ErrorKind::Unsupported {
            //     return Err(err);
            // }

            // // otherwise, trigger read & cache
            drop(cache);
            self.read(object_name.clone())?;
        } else {
            drop(cache);
        }

        let cache = self.cache.borrow();
        log::trace!("partial_write: emulate");
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        let cached_obj = cached_obj_ref.unwrap().1.borrow();
        let old_buffer: Vec<u8> = cached_obj.data.clone();
        drop(cached_obj);
        drop(cached_obj_ref);
        drop(cache);

        let mut new_buffer: Vec<u8> = Vec::new();

        // Patch it partially
        if offset > 0 {
            new_buffer.extend_from_slice(&old_buffer[0..(offset as usize)]);
        }
        new_buffer.extend_from_slice(data);
        let remaining = old_buffer.len() - (offset as usize) - length;
        if remaining > 0 {
            new_buffer.extend_from_slice(&old_buffer[((offset as usize)+length)..((offset as usize)+length+remaining)]);
        }

        // Put back in place
        self.write(object_name, &new_buffer)
    }
}

impl StreamingObjectStorage for CacheBackend {}
impl StreamingPartialAccessObjectStorage for CacheBackend {}
impl ObjectStorage for CacheBackend {}
