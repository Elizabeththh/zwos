use alloc::sync::Arc;
use core::fmt;

use lru::LruCache;
use spin::Mutex;
use storage::{BlockTrait, Block512, BlockDevice, CacheManager, CacheBlock, CachedDevice};

use core::num::NonZeroUsize;

pub struct LruCacheManager<B: BlockTrait> {
    cache: Mutex<LruCache<usize, CacheBlock<B>>>,
    cap: usize,
}

impl<B: BlockTrait> LruCacheManager<B> {
    pub fn new(capacity: usize) -> Self {
        let cap_nz = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(16).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(cap_nz)),
            cap: capacity,
        }
    }
}

impl<B: BlockTrait> CacheManager<B> for LruCacheManager<B> {
    fn read(&self, offset: usize) -> Option<B> {
        let mut cache = self.cache.lock();
        cache.get(&offset).map(|cb| cb.data.clone())
    }

    fn insert(&self, block: CacheBlock<B>) -> Option<CacheBlock<B>> {
        let mut cache = self.cache.lock();
        let offset = block.offset;

        if cache.contains(&offset) {
            let old = cache.put(offset, block);
            return old;
        }

        if cache.len() >= self.cap {
            let evicted = cache.pop_lru();
            cache.put(offset, block);
            return evicted.map(|(_, v)| v);
        }

        cache.put(offset, block);
        None
    }

    fn capacity(&self) -> usize {
        self.cap
    }

    fn len(&self) -> usize {
        self.cache.lock().len()
    }

    fn dirty_count(&self) -> usize {
        let cache = self.cache.lock();
        cache.iter().filter(|(_, cb)| cb.dirty).count()
    }
}

impl<B: BlockTrait> fmt::Debug for LruCacheManager<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cache = self.cache.lock();
        f.debug_struct("LruCacheManager")
            .field("capacity", &self.cap)
            .field("len", &cache.len())
            .finish()
    }
}

pub fn create_cached_device(
    device: impl BlockDevice<Block512>,
    cache_capacity: usize,
) -> CachedDevice<Block512, LruCacheManager<Block512>> {
    let device_arc: Arc<dyn BlockDevice<Block512>> = Arc::new(device);
    let cache = LruCacheManager::new(cache_capacity);
    CachedDevice::new(cache, device_arc)
}

pub fn wrap_cached(
    device_arc: Arc<dyn BlockDevice<Block512>>,
    cache_capacity: usize,
) -> CachedDevice<Block512, LruCacheManager<Block512>> {
    let cache = LruCacheManager::new(cache_capacity);
    CachedDevice::new(cache, device_arc)
}

pub fn cache_stats<B: BlockTrait, C: CacheManager<B>>(cached: &CachedDevice<B, C>) -> (usize, usize, usize) {
    (cached.cache.capacity(), cached.cache.len(), cached.cache.dirty_count())
}