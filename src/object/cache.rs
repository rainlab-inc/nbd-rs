use std::{
    io::{Error, ErrorKind},
    time::{Instant, Duration},
    collections::{HashMap},
    ops::{Deref},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{Sender, channel},
        Arc, RwLock, Mutex
    },
    thread,
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

struct CacheValRef(Arc<RwLock<CachedObject>>);
type CacheMap = HashMap<String, CacheValRef>;
struct CacheMapRef(Arc<RwLock<CacheMap>>);

impl CacheValRef {
    pub fn new(obj: CachedObject) -> CacheValRef {
        CacheValRef(Arc::new(
            RwLock::new(obj)
        ))
    }
}

impl Deref for CacheValRef {
    type Target = Arc<RwLock<CachedObject>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for CacheValRef {
    fn clone(&self) -> CacheValRef {
        CacheValRef(Arc::clone(&self.0))
    }
}

// impl DerefMut for CacheValRef {
//     fn deref_mut(&mut self) -> Self::Target {
//         self.0.write().unwrap()
//     }
// }

impl CacheMapRef {
    pub fn new() -> CacheMapRef {
        CacheMapRef(Arc::new(
            RwLock::new(
                HashMap::<String, CacheValRef>::new()
            )
        ))
        // RefCell::<HashMap<String, Rc<RefCell<CachedObject>>>>::new(
        //     HashMap::<String, Rc<RefCell<CachedObject>>>::new()
        // )
    }
}

impl Clone for CacheMapRef {
    fn clone(&self) -> CacheMapRef {
        CacheMapRef(Arc::clone(&self.0))
    }
}

impl Deref for CacheMapRef {
    type Target = Arc<RwLock<CacheMap>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct CacheBackend {
    config: String,
    read_backend: Arc<Mutex<Box<dyn ObjectStorage>>>,
    write_backend: Arc<Mutex<Box<dyn ObjectStorage>>>,
    cache: CacheMapRef,
    mem_usage: Arc<AtomicUsize>,
    mem_limit: usize,
    stall_secs: u16,
    bgthread_jh: Option<thread::JoinHandle<()>>,
    sender: Option<Sender<bool>>
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

        let mut obj = CacheBackend {
            config: config.clone(),
            read_backend: Arc::new(Mutex::new(object_storage_with_config(config.clone()).unwrap())),
            write_backend: Arc::new(Mutex::new(object_storage_with_config(config.clone()).unwrap())),
            cache: CacheMapRef::new(),
            mem_usage: Arc::new(AtomicUsize::new(0)),
            mem_limit: 128 * 1024 * 1024,
            stall_secs: 3, // persist things to disk after 3 seconds
            bgthread_jh: None,
            sender: None,
        };
        obj.start_persister();
        obj
    }

    // TODO fn drop: send quit req to bgthread and join that here

    fn start_persister(&mut self) {
        let cache_ref = self.cache.clone();
        let mem_limit = self.mem_limit;
        let stall_secs = self.stall_secs;
        let mem_usage = Arc::clone(&self.mem_usage);
        let write_backend = Arc::clone(&self.write_backend);
        let (send, rcv) = channel();
        self.sender = Some(send);

        self.bgthread_jh = Some(thread::Builder::new()
            .spawn(move || {
                loop {
                    let cache = cache_ref.read().unwrap();
                    let total_pages = cache.len();
                    let unwritten_pages = cache.iter()
                        .filter(|(k, cref)| {
                            let c = cref.read().unwrap();
                            c.writes > c.persists
                        }) // only not-persisted ones
                        .count();

                    let oldest_unwritten_page = cache.iter()
                        .filter(|(k, cref)| {
                            let c = cref.read().unwrap();
                            c.writes > c.persists && c.last_write.unwrap().elapsed() > Duration::from_secs(stall_secs.into())
                        }) // only not-persisted ones, after `stall_secs`
                        .min_by(|(xk, xref), (yk, yref)| {
                            let x = xref.read().unwrap();
                            let y = yref.read().unwrap();
                            x.last_write.unwrap().cmp(&y.last_write.unwrap())
                        });

                    // skip
                    if oldest_unwritten_page.is_none() {
                        drop(oldest_unwritten_page);
                        drop(cache);
                        //thread::sleep(Duration::from_millis(5000));
                        log::debug!("mem_usage: {}, {} pages, {} unwritten", mem_usage.load(Ordering::Acquire), total_pages, unwritten_pages);
                        match rcv.recv_timeout(Duration::from_secs(5)).unwrap_or(true) {
                            true => continue,
                            false => break
                        };
                    }

                    let cache_kp = oldest_unwritten_page.unwrap();
                    let obj_name = cache_kp.0.to_string();
                    let obj_ref_clone = Arc::clone(cache_kp.1);
                    let mut cache_obj = obj_ref_clone.write().unwrap();
                    drop(cache_kp);
                    drop(cache);

                    let backend = write_backend.lock().unwrap();
                    let write_res = backend.write(obj_name.clone(), &cache_obj.data.clone());
                    if write_res.is_ok() {
                        let persist_res = backend.persist_object(obj_name.clone());
                        if persist_res.is_ok() {
                            cache_obj.persists = cache_obj.writes;
                            cache_obj.last_persist = Some(Instant::now());
                        } else {
                            log::warn!("background persist (after write) failed");
                        }
                        log::debug!("mem_usage: {}, left {} pages, {} unwritten, wrote 1 page", mem_usage.load(Ordering::Acquire), total_pages, unwritten_pages - 1);
                    } else {
                        log::warn!("background write failed, will retry");
                    }
                    drop(backend);
                }
            })
            .unwrap());
    }

    fn least_important_cache_key(cache_ref: CacheMapRef) -> Option<String> {
        let cache = cache_ref.read().unwrap();
        let kvpair = cache
            .iter()
            .filter(|(k, cref)| {
                let c = cref.read().unwrap();
                c.writes <= c.persists
            }) // only already persisted ones
            .min_by(|(xk, xref), (yk, yref)| {
                let x = xref.read().unwrap();
                let y = yref.read().unwrap();
                if x.last_read.is_none() {
                    return std::cmp::Ordering::Less;
                }
                if y.last_read.is_none() {
                    return std::cmp::Ordering::Greater;
                }

                x.last_read.unwrap().cmp(&y.last_read.unwrap())
            });

        if kvpair.is_some() {
            let (k, vref) = kvpair.unwrap();
            return Some(k.to_string());
        }

        return None;
    }

    fn get_cache(&self, key: String) -> Option<CacheValRef> {
        let cache = self.cache.read().unwrap();
        let cache_entry = cache.get_key_value(&key);
        if cache_entry.is_none() {
            return None;
        }

        let (kref, cref) = cache_entry.unwrap();
        Some(cref.clone())
    }

    fn ensure_free_memory(&self, bytes: usize) -> Result<(), Error> {
        while self.mem_usage.load(Ordering::Acquire) + bytes >= self.mem_limit {
            // .. free least important object
            let victim_key_res = CacheBackend::least_important_cache_key(self.cache.clone());
            if victim_key_res.is_none() {
                // TODO: Consider blocking here and persist some write cache, to make space.
                return Err(Error::new(ErrorKind::Other, "Cannot free memory"));
            }

            let victim_key = victim_key_res.unwrap();
            let cref = self.get_cache(victim_key.clone()).unwrap();
            let c = cref.read().unwrap();

            self.mem_usage.fetch_sub(c.size, Ordering::Release);
            log::debug!("mem: removing object {}, mem_usage to be: {}", victim_key.clone(), self.mem_usage.load(Ordering::Acquire));
            let mut cache = self.cache.write().unwrap();
            cache.remove(&victim_key);
        }

        Ok(())
    }
}

fn retry<F, T>(mut op: F) -> Result<T, Error>
where
    F: FnMut() -> Result<T, Error>,
{
    let mut retries = 3;
    loop {
        let res = op();
        if res.is_ok() {
            return res;
        }

        let err = res.err().unwrap();
        // only retry for 'ErrorKind::Other'
        if err.kind() != ErrorKind::Other {
            return Err(err);
        }

        thread::sleep(Duration::from_secs(1));
        retries -= 1;
        if retries == 0 {
            return Err(err);
        }

        log::warn!("retrying");
    }
}

impl SimpleObjectStorage for CacheBackend {
    fn init(&mut self, conn_str: String) {
        // .. noop
        log::info!("init");
    }

    fn exists(&self, object_name: String) -> Result<bool, Error> {
        let cache = self.cache.read().unwrap();
        if cache.contains_key(&object_name.clone()) {
            log::trace!("exists: hit");
            return Ok(true);
        }

        log::trace!("exists: miss");
        // TODO: Cache exists|not status as well?

        retry(|| {
            self.read_backend.lock().unwrap().exists(object_name.clone())
        })
    }

    fn read(&self, object_name: String) -> Result<Vec<u8>, Error> {
        let cache = self.cache.read().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("read: hit");
            let mut cached_obj = cached_obj_ref.unwrap().1.write().unwrap();
            cached_obj.reads += 1;
            cached_obj.last_read = Some(Instant::now());
            return Ok(cached_obj.data.clone());
        }
        drop(cached_obj_ref);
        drop(cache);

        log::trace!("read: miss");
        let data = retry(|| {
            self.read_backend.lock().unwrap().read(object_name.clone())
        })?;

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

        self.ensure_free_memory(cached_object.size);
        let mut cache = self.cache.write().unwrap();
        self.mem_usage.fetch_add(cached_object.size, Ordering::Release);
        cache.insert(object_name.clone(), CacheValRef::new(cached_object));
        Ok(data)
    }

    fn write(&self, object_name: String, data: &[u8]) -> Result<Propagation, Error> {
        let mut cache = self.cache.write().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("write: hit");
            let mut cached_obj = cached_obj_ref.unwrap().1.write().unwrap();
            cached_obj.writes += 1;
            cached_obj.last_write = Some(Instant::now());
            cached_obj.data = data.to_vec();
            log::trace!("mem_usage: {}", self.mem_usage.load(Ordering::Acquire));
            self.sender.as_ref().unwrap().send(true);
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
        // TODO: Check if mem limit allows this, otherwise
        // * block until it allows, and pressure cache purge
        self.ensure_free_memory(cached_object.size);
        self.mem_usage.fetch_add(cached_object.size, Ordering::Release);
        log::trace!("mem_usage: {}", self.mem_usage.load(Ordering::Acquire));
        cache.insert(object_name.clone(), CacheValRef::new(cached_object));
        self.sender.as_ref().unwrap().send(true);
        Ok(Propagation::Queued)
    }

    fn delete(&self, object_name: String) -> Result<Propagation, Error> {
        let mut cache = self.cache.write().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let cached_obj = cached_obj_ref.unwrap().1.read().unwrap();
            self.mem_usage.fetch_sub(cached_obj.size, Ordering::Release);
            log::trace!("mem_usage: {}", self.mem_usage.load(Ordering::Acquire));
        }
        cache.remove(&object_name.clone());

        self.write_backend.lock().unwrap().delete(object_name.clone())
    }

    fn get_size(&self, object_name: String) -> Result<u64, Error> {
        let cache = self.cache.read().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("size: hit");
            let cached_obj = cached_obj_ref.unwrap().1.read().unwrap();
            return Ok(cached_obj.size as u64);
        }

        log::trace!("size: miss");
        // TODO: cache this(size only) as well??
        self.read_backend.lock().unwrap().get_size(object_name.clone())
    }

    fn start_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // increase
        let cache = self.cache.write().unwrap();
        if !cache.contains_key(&object_name.clone()) {
            // for side effect;
            self.read(object_name.clone())?;
        }

        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.write().unwrap();
            cached_obj.keep += 1;
        }

        self.write_backend.lock().unwrap().start_operations_on_object(object_name.clone())
    }

    fn end_operations_on_object(&self, object_name: String) -> Result<(), Error> {
        // decrease
        let cache = self.cache.write().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.write().unwrap();
            cached_obj.keep -= 1;
        }

        self.write_backend.lock().unwrap().end_operations_on_object(object_name.clone())
    }

    fn persist_object(&self, object_name: String) -> Result<Propagation, Error> {
        let cache = self.cache.write().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        let backend = self.write_backend.lock().unwrap();
        if cached_obj_ref.is_some() {
            let mut cached_obj = cached_obj_ref.unwrap().1.write().unwrap();

            // already persisted
            if cached_obj.persists == cached_obj.writes {
                return Ok(Propagation::Redundant);
            }

            log::debug!("persist: hit");
            let write_propagation = retry(|| {
                backend.write(object_name.clone(), &cached_obj.data.clone())
            })?;
            cached_obj.persists = cached_obj.writes;
            cached_obj.last_persist = Some(Instant::now());

            let persist_propagation = retry(|| {
                backend.persist_object(object_name.clone())
            })?;

            if (persist_propagation as u8) > (write_propagation as u8) {
                return Ok(persist_propagation);
            }

            return Ok(write_propagation);
        }

        retry(|| { backend.persist_object(object_name.clone()) })
    }

    fn trim_object (&self, object_name: String, offset: u64, length: usize) -> Result<Propagation, Error> {
        let cache = self.cache.write().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("write: hit");
            let mut cached_obj = cached_obj_ref.unwrap().1.write().unwrap();
            cached_obj.writes += 1;
            cached_obj.last_write = Some(Instant::now());
            let mut trimmed_data = vec![];
            {
                trimmed_data.extend_from_slice(&cached_obj.data[..(offset as usize)]);
                trimmed_data.extend_from_slice(&cached_obj.data[(offset as usize + length)..]);
            }
            cached_obj.data = trimmed_data;
            log::trace!("mem_usage: {}", self.mem_usage.load(Ordering::Acquire));
            self.sender.as_ref().unwrap().send(true);
            return Ok(Propagation::Queued);
        }
        log::trace!("write: miss");
        self.sender.as_ref().unwrap().send(true);
        Ok(Propagation::Ignored) // Fail/Ignore/Redundant?
    }

    fn close(&mut self) {
        log::debug!("object::cache::close");
        self.sender.as_ref().unwrap().send(false);
        match self.bgthread_jh.take() {
            Some(bgthread) => bgthread.join().unwrap(),
            None => log::debug!("no thread!")
        };
    }
}

impl PartialAccessObjectStorage for CacheBackend {

    fn partial_read(&self, object_name: String, offset: u64, length: usize) -> Result<Vec<u8>, Error> {
        let cache = self.cache.write().unwrap();
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        if cached_obj_ref.is_some() {
            log::trace!("partial_read: hit");
            let cached_obj = cached_obj_ref.unwrap().1.read().unwrap();
            let data: Vec<u8> = cached_obj.data.clone();
            let slice: Vec<u8> = data[(offset as usize)..((offset as usize) + length)].to_vec();
            return Ok(slice);
        }
        drop(cache);

        // // not cached; try backend
        log::trace!("partial_read: miss");
        // let backend_read_res = self.read_backend.partial_read(object_name.clone(), offset, length);
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
        let cache = self.cache.read().unwrap();
        if !cache.contains_key(&object_name.clone()) {
            // code below commented out, because if we proxy this to backend,
            // and if the backend has an ugly workaround for this,
            // the client will be repeatedly using the ugly backend for this,
            // when the cache would have been much better..

            // log::trace!("partial_write: miss -> proxy");
            // // try proxy
            // let backend_write_res = self.write_backend.partial_write(object_name.clone(), offset, length, data);
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

        let cache = self.cache.read().unwrap();
        log::trace!("partial_write: emulate");
        let cached_obj_ref = cache.get_key_value(&object_name.clone());
        let cached_obj = cached_obj_ref.unwrap().1.read().unwrap();
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
