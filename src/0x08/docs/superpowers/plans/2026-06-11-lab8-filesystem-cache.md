# Lab 8: Writable TmpFS & Block Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a writable temporary filesystem (TmpFS) with hard links, a VFS for multiple mounts, a RamDisk block device, and a block cache layer with LRU eviction.

**Architecture:** TmpFS is a simple inode-based filesystem (32-byte inodes, 32-byte dir entries, 512-byte blocks) stored on a RamDisk (frame-allocated memory). VFS routes paths to appropriate mounts. Cache layer wraps block devices with LRU-based CacheManager, auto write-back on Drop.

**Tech Stack:** Rust no_std, storage crate (ysos_storage), kernel crate (ysos_kernel), lru crate, spin crate

---

## Task 1: Storage Crate Foundation Changes

**Files:**
- Modify: `crates/storage/src/common/io.rs` (write_all)
- Modify: `crates/storage/src/common/filesystem.rs` (create_dir, link)
- Modify: `crates/storage/src/common/metadata.rs` (links field)
- Modify: `crates/storage/src/common/mount.rs` (forward all FileSystem methods)
- Modify: `crates/storage/src/common/mod.rs` (add cache module export)
- Modify: `crates/storage/src/fs/mod.rs` (add tmpfs module)

### Step 1: Implement write_all in io.rs

Replace `todo!()` with actual implementation in `Write::write_all`.

### Step 2: Add create_dir and link to FileSystem trait

Add optional methods with default `NotSupported` return.

### Step 3: Add links field to Metadata

Add `pub links: usize` with default 1 in `new()`.

### Step 4: Forward all FileSystem methods in Mount

Add forwarding for create_file, append_file, remove_file, remove_dir, create_dir, link, copy_file, move_file, move_dir.

### Step 5: Add tmpfs module to fs/mod.rs

---

## Task 2: TmpFS Filesystem Implementation

**Files:**
- Create: `crates/storage/src/fs/tmpfs/mod.rs`
- Create: `crates/storage/src/fs/tmpfs/superblock.rs`
- Create: `crates/storage/src/fs/tmpfs/inode.rs`
- Create: `crates/storage/src/fs/tmpfs/direntry.rs`
- Create: `crates/storage/src/fs/tmpfs/file.rs`
- Create: `crates/storage/src/fs/tmpfs/impls.rs`

### On-disk layout (512-byte blocks):
- Block 0: Superblock (magic=0x544D5046, version, block_size, total_blocks, inode_count, bitmap_start, bitmap_blocks, inode_start, inode_blocks, data_start, data_blocks, free_data_blocks)
- Block 1: Allocation bitmap (1 bit per data block)
- Blocks 2..18: Inode table (16 inodes per block, 256 inodes total)
- Blocks 18..end: Data blocks

### Inode (32 bytes):
- type(u8): 0=free, 1=file, 2=dir
- size(u32): file size
- direct_blocks([u16;10]): data block indices relative to data_start
- links_count(u8)
- _reserved([u8;5])

### Dir entry (32 bytes):
- name([u8;28]): null-terminated
- inode(u16)
- _reserved([u8;2])

---

## Task 3: RamDisk & VFS in Kernel

**Files:**
- Create: `crates/kernel/src/drivers/ramdisk.rs`
- Modify: `crates/kernel/src/drivers/filesystem.rs` (replace ROOTFS with VFS)
- Modify: `crates/kernel/src/drivers/mod.rs` (add ramdisk module)
- Modify: `crates/kernel/src/lib.rs` (updated init sequence)
- Modify: `Makefile` (increase QEMU memory to 128M)

### RamDisk: Frame-allocated memory-backed BlockDevice<Block512>
- Vec of virtual addresses of allocated frames
- 8 blocks per frame (4096/512)
- Read/write via direct memory access

### VFS: VirtualFileSystem with Vec<Mount>
- find_mount(): longest prefix match
- read_dir("/"): synthesize mount point entries
- Implement FileSystem trait

---

## Task 4: Init Sequence & Task Goals

Mount boot at /boot, format RamDisk with TmpFS, mount at /tmp.
Create /tmp/mydir, /tmp/mydir/hello.txt, write student ID, read & print.

---

## Task 5: Hard Links Bonus

Add link() to TmpFS: same inode, multiple dir entries, links_count tracking.

---

## Task 6: Cache Layer (storage crate)

**Files:**
- Create: `crates/storage/src/common/cache.rs`
- Modify: `crates/storage/src/common/mod.rs` (export cache)

### CacheManager trait, CacheBlock struct, CachedDevice struct
- CacheBlock: data, dirty flag, offset, device Arc; Drop writes back
- CacheManager: read, insert, capacity, len, dirty_count
- CachedDevice: implements BlockDevice<B> using cache + device

---

## Task 7: Cache Implementation (kernel)

**Files:**
- Create: `crates/kernel/src/drivers/cache.rs`
- Modify: `crates/kernel/src/drivers/mod.rs` (add cache module)
- Modify: `crates/kernel/Cargo.toml` (add lru dependency)
- Modify: `crates/kernel/src/drivers/filesystem.rs` (wrap device with cache)

### LruCacheManager using lru crate with spin::Mutex
- Wrap CachedDevice around AtaDrive/Partition before Fat16

---

## Task 8: Cache Stats & Performance Bonus

- Expose cache stats (capacity, len, dirty_count) in system status
- Benchmark cached vs uncached reads