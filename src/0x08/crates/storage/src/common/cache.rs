use alloc::sync::Arc;
use core::fmt::Debug;

use crate::*;

pub trait CacheManager<B: BlockTrait>: Debug + Send + Sync + 'static {
    fn read(&self, offset: usize) -> Option<B>;
    fn insert(&self, block: CacheBlock<B>) -> Option<CacheBlock<B>>;
    fn capacity(&self) -> usize;
    fn len(&self) -> usize;
    fn dirty_count(&self) -> usize;
}

pub struct CacheBlock<B: BlockTrait> {
    pub data: B,
    pub dirty: bool,
    pub offset: usize,
    device: Arc<dyn BlockDevice<B>>,
}

impl<B: BlockTrait> CacheBlock<B> {
    pub fn new(data: B, offset: usize, device: Arc<dyn BlockDevice<B>>) -> Self {
        Self {
            data,
            dirty: false,
            offset,
            device,
        }
    }

    pub fn new_dirty(data: B, offset: usize, device: Arc<dyn BlockDevice<B>>) -> Self {
        Self {
            data,
            dirty: true,
            offset,
            device,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

impl<B: BlockTrait> Drop for CacheBlock<B> {
    fn drop(&mut self) {
        if self.dirty {
            if let Err(e) = self.device.write_block(self.offset, &self.data) {
                error!("Cache write-back error at block {}: {:?}", self.offset, e);
            } else {
                trace!("Cache write-back: block {}", self.offset);
            }
        }
    }
}

impl<B: BlockTrait> Debug for CacheBlock<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CacheBlock")
            .field("offset", &self.offset)
            .field("dirty", &self.dirty)
            .finish()
    }
}

pub struct CachedDevice<B, C>
where
    B: BlockTrait,
    C: CacheManager<B>,
{
    pub cache: C,
    device: Arc<dyn BlockDevice<B>>,
}

impl<B, C> CachedDevice<B, C>
where
    B: BlockTrait,
    C: CacheManager<B>,
{
    pub fn new(cache: C, device: Arc<dyn BlockDevice<B>>) -> Self {
        Self { cache, device }
    }
}

impl<B, C> BlockDevice<B> for CachedDevice<B, C>
where
    B: BlockTrait,
    C: CacheManager<B>,
{
    fn block_count(&self) -> FsResult<usize> {
        self.device.block_count()
    }

    fn read_block(&self, offset: usize, block: &mut B) -> FsResult {
        if let Some(cached) = self.cache.read(offset) {
            block.clone_from(&cached);
            return Ok(());
        }

        self.device.read_block(offset, block)?;

        let cache_block = CacheBlock::new(block.clone(), offset, self.device.clone());
        if let Some(evicted) = self.cache.insert(cache_block) {
            drop(evicted);
        }

        Ok(())
    }

    fn write_block(&self, offset: usize, block: &B) -> FsResult {
        let cache_block = CacheBlock::new_dirty(block.clone(), offset, self.device.clone());
        if let Some(evicted) = self.cache.insert(cache_block) {
            drop(evicted);
        }

        Ok(())
    }
}

impl<B, C> Debug for CachedDevice<B, C>
where
    B: BlockTrait,
    C: CacheManager<B>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CachedDevice")
            .field("cache", &self.cache)
            .finish()
    }
}