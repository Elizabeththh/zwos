use super::*;

impl TmpFsImpl {
    pub fn new(inner: impl BlockDevice<Block512>) -> Self {
        let mut block = Block512::default();
        inner.read_block(0, &mut block).unwrap();
        let sb = Superblock::new(block.as_ref()).unwrap();
        trace!("Loading TmpFS Volume: {:#?}", sb);
        Self {
            sb,
            inner: Box::new(inner),
        }
    }

    pub fn format<B: BlockDevice<Block512>>(
        device: &B,
        total_blocks: usize,
        inode_count: usize,
    ) -> FsResult {
        let inode_blocks = (inode_count + INODES_PER_BLOCK - 1) / INODES_PER_BLOCK;
        let bitmap_start = 1u32;
        let data_blocks_estimated = total_blocks as u32 - 1 - 1 - inode_blocks as u32;
        let bits_needed = data_blocks_estimated;
        let bitmap_blocks = ((bits_needed as usize) + 4095) / 4096;
        let bitmap_blocks = core::cmp::max(bitmap_blocks, 1) as u32;
        let inode_start = bitmap_start + bitmap_blocks;
        let data_start = inode_start + inode_blocks as u32;
        let data_blocks = total_blocks as u32 - data_start;

        let mut sb = Superblock::empty();
        sb.set_magic(TMPFS_MAGIC);
        sb.set_version(TMPFS_VERSION);
        sb.set_block_size(BLOCK_SIZE as u32);
        sb.set_total_blocks(total_blocks as u32);
        sb.set_inode_count(inode_count as u32);
        sb.set_bitmap_start(bitmap_start);
        sb.set_bitmap_blocks(bitmap_blocks);
        sb.set_inode_start(inode_start);
        sb.set_inode_blocks(inode_blocks as u32);
        sb.set_data_start(data_start);
        sb.set_data_blocks(data_blocks);
        sb.set_free_data_blocks(data_blocks);
        sb.set_next_free_inode(1);

        let mut block = Block512::default();
        block.as_mut()[..512].copy_from_slice(&sb.data);
        device.write_block(0, &block)?;

        for i in bitmap_start..(bitmap_start + bitmap_blocks) {
            let mut b = Block512::default();
            device.write_block(i as usize, &b)?;
        }

        for i in inode_start as usize..data_start as usize {
            let mut b = Block512::default();
            device.write_block(i, &b)?;
        }

        let mut root_inode = InodeData::new_dir();
        let mut inode_block = Block512::default();
        inode_block.as_mut()[..INODE_SIZE].copy_from_slice(&root_inode.data);
        device.write_block(inode_start as usize, &inode_block)?;

        Ok(())
    }

    pub fn read_inode(&self, idx: u16) -> FsResult<InodeData> {
        let inode_count = self.sb.inode_count() as usize;
        if idx as usize >= inode_count {
            return Err(FsError::InvalidOffset);
        }

        let block_offset = idx as usize / INODES_PER_BLOCK;
        let offset_in_block = (idx as usize % INODES_PER_BLOCK) * INODE_SIZE;
        let block_num = self.sb.inode_start() as usize + block_offset;

        let mut block = Block512::default();
        self.inner.read_block(block_num, &mut block)?;

        let inode = InodeData {
            data: block.as_ref()[offset_in_block..offset_in_block + INODE_SIZE]
                .try_into()
                .unwrap(),
        };

        Ok(inode)
    }

    pub fn write_inode(&self, idx: u16, inode: &InodeData) -> FsResult {
        let block_offset = idx as usize / INODES_PER_BLOCK;
        let offset_in_block = (idx as usize % INODES_PER_BLOCK) * INODE_SIZE;
        let block_num = self.sb.inode_start() as usize + block_offset;

        let mut block = Block512::default();
        self.inner.read_block(block_num, &mut block)?;
        block.as_mut()[offset_in_block..offset_in_block + INODE_SIZE]
            .copy_from_slice(&inode.data);
        self.inner.write_block(block_num, &block)?;

        Ok(())
    }

    pub fn allocate_data_block(&self) -> FsResult<u16> {
        let bitmap_start = self.sb.bitmap_start() as usize;
        let data_start = self.sb.data_start() as usize;
        let data_blocks = self.sb.data_blocks() as usize;

        let mut block = Block512::default();
        self.inner.read_block(bitmap_start, &mut block)?;

        for byte_idx in 0..data_blocks {
            let bit_byte = byte_idx / 8;
            let bit_pos = byte_idx % 8;
            if bit_byte >= BLOCK_SIZE {
                break;
            }
            if block.as_ref()[bit_byte] & (1 << bit_pos) == 0 {
                block.as_mut()[bit_byte] |= 1 << bit_pos;
                self.inner.write_block(bitmap_start, &block)?;
                let rel = (byte_idx) as u16 + 1;
                return Ok(rel);
            }
        }

        Err(FsError::NotSupported)
    }

    pub fn free_data_block(&self, rel: u16) -> FsResult {
        if rel == 0 {
            return Ok(());
        }
        let bitmap_start = self.sb.bitmap_start() as usize;
        let byte_idx = (rel - 1) as usize;
        let bit_byte = byte_idx / 8;
        let bit_pos = byte_idx % 8;

        let mut block = Block512::default();
        self.inner.read_block(bitmap_start, &mut block)?;
        block.as_mut()[bit_byte] &= !(1 << bit_pos);
        self.inner.write_block(bitmap_start, &block)?;

        Ok(())
    }

    pub fn allocate_inode(&self) -> FsResult<u16> {
        let inode_count = self.sb.inode_count() as usize;
        for idx in 1..inode_count {
            let inode = self.read_inode(idx as u16)?;
            if inode.is_free() {
                return Ok(idx as u16);
            }
        }
        Err(FsError::NotSupported)
    }

    pub fn add_dir_entry(&self, dir_inode_idx: u16, name: &str, child_inode: u16) -> FsResult {
        let mut dir_inode = self.read_inode(dir_inode_idx)?;
        let entry = DirEntryData::new(name, child_inode);

        let dir_size = dir_inode.size() as usize;
        let n_blocks = (dir_size + BLOCK_SIZE - 1) / BLOCK_SIZE;

        if dir_size % DIRENTRY_SIZE == 0 && dir_size > 0 {
            let block_idx = dir_size / BLOCK_SIZE;
            if block_idx >= INODE_DIRECT_BLOCKS {
                return Err(FsError::NotSupported);
            }
            if dir_inode.direct_block(block_idx) == 0 {
                let rel = self.allocate_data_block()?;
                dir_inode.set_direct_block(block_idx, rel);
                self.write_inode(dir_inode_idx, &dir_inode)?;
            }
        }

        let entry_offset_in_dir = dir_size;
        let block_idx_in_dir = entry_offset_in_dir / BLOCK_SIZE;

        if block_idx_in_dir >= INODE_DIRECT_BLOCKS {
            return Err(FsError::NotSupported);
        }

        let mut data_block_rel = dir_inode.direct_block(block_idx_in_dir);
        if data_block_rel == 0 {
            let rel = self.allocate_data_block()?;
            dir_inode.set_direct_block(block_idx_in_dir, rel);
            data_block_rel = rel;
        }

        let actual_block = self.sb.data_start() as usize + data_block_rel as usize;
        let offset_in_block = entry_offset_in_dir % BLOCK_SIZE;

        let mut block = Block512::default();
        self.inner.read_block(actual_block, &mut block)?;
        block.as_mut()[offset_in_block..offset_in_block + DIRENTRY_SIZE]
            .copy_from_slice(&entry.data);
        self.inner.write_block(actual_block, &block)?;

        dir_inode.set_size(dir_size as u32 + DIRENTRY_SIZE as u32);
        self.write_inode(dir_inode_idx, &dir_inode)?;

        Ok(())
    }

    pub fn find_dir_entry(&self, dir_inode_idx: u16, name: &str) -> FsResult<u16> {
        let dir_inode = self.read_inode(dir_inode_idx)?;
        let dir_size = dir_inode.size() as usize;
        let n_blocks = (dir_size + BLOCK_SIZE - 1) / BLOCK_SIZE;

        for block_idx in 0..n_blocks {
            if block_idx >= INODE_DIRECT_BLOCKS {
                break;
            }
            let data_block_rel = dir_inode.direct_block(block_idx);
            if data_block_rel == 0 {
                continue;
            }

            let actual_block = self.sb.data_start() as usize + data_block_rel as usize;
            let mut block = Block512::default();
            self.inner.read_block(actual_block, &mut block)?;

            let entries_in_block = BLOCK_SIZE / DIRENTRY_SIZE;
            for e in 0..entries_in_block {
                let off = e * DIRENTRY_SIZE;
                if off >= dir_size - block_idx * BLOCK_SIZE && dir_size > 0 {
                    break;
                }
                let entry = DirEntryData {
                    data: block.as_ref()[off..off + DIRENTRY_SIZE].try_into().unwrap(),
                };
                if entry.is_empty() {
                    continue;
                }
                if entry.name() == name {
                    return Ok(entry.inode());
                }
            }
        }

        Err(FsError::FileNotFound)
    }

    pub fn resolve_path(&self, path: &str) -> FsResult<u16> {
        let parts: Vec<&str> = path
            .split(PATH_SEPARATOR)
            .filter(|p| !p.is_empty())
            .collect();

        if parts.is_empty() {
            return Ok(0);
        }

        let mut current_inode = 0u16;
        for part in &parts {
            let inode = self.read_inode(current_inode)?;
            if !inode.is_dir() {
                return Err(FsError::NotADirectory);
            }
            current_inode = self.find_dir_entry(current_inode, part)?;
        }

        Ok(current_inode)
    }

    pub fn list_dir(&self, dir_inode_idx: u16) -> FsResult<Vec<Metadata>> {
        let dir_inode = self.read_inode(dir_inode_idx)?;
        if !dir_inode.is_dir() {
            return Err(FsError::NotADirectory);
        }

        let dir_size = dir_inode.size() as usize;
        let n_blocks = (dir_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let mut entries = Vec::new();

        for block_idx in 0..n_blocks {
            if block_idx >= INODE_DIRECT_BLOCKS {
                break;
            }
            let data_block_rel = dir_inode.direct_block(block_idx);
            if data_block_rel == 0 {
                continue;
            }

            let actual_block = self.sb.data_start() as usize + data_block_rel as usize;
            let mut block = Block512::default();
            self.inner.read_block(actual_block, &mut block)?;

            let entries_in_block = BLOCK_SIZE / DIRENTRY_SIZE;
            let remaining = dir_size - block_idx * BLOCK_SIZE;
            let max_entries = core::cmp::min(entries_in_block, remaining / DIRENTRY_SIZE);

            for e in 0..max_entries {
                let off = e * DIRENTRY_SIZE;
                let de = DirEntryData {
                    data: block.as_ref()[off..off + DIRENTRY_SIZE].try_into().unwrap(),
                };
                if de.is_empty() {
                    continue;
                }
                let child_inode = self.read_inode(de.inode())?;
                let entry_type = if child_inode.is_dir() {
                    FileType::Directory
                } else {
                    FileType::File
                };
                entries.push(Metadata::new(
                    String::from(de.name()),
                    entry_type,
                    child_inode.size() as usize,
                    None,
                    None,
                    None,
                ));
            }
        }

        Ok(entries)
    }

    fn root_metadata(&self) -> Metadata {
        Metadata::new(String::from("/"), FileType::Directory, 0, None, None, None)
    }
}

impl FileSystem for TmpFs {
    fn read_dir(&self, path: &str) -> FsResult<Box<dyn Iterator<Item = Metadata> + Send>> {
        let dir_inode_idx = self.handle.resolve_path(path)?;
        let entries = self.handle.list_dir(dir_inode_idx)?;
        Ok(Box::new(entries.into_iter()))
    }

    fn open_file(&self, path: &str) -> FsResult<FileHandle> {
        let inode_idx = self.handle.resolve_path(path)?;
        let inode = self.handle.read_inode(inode_idx)?;
        if !inode.is_file() {
            return Err(FsError::NotAFile);
        }

        let meta = Metadata::new(
            String::from(path.rsplit('/').next().unwrap_or(path)),
            FileType::File,
            inode.size() as usize,
            None,
            None,
            None,
        );

        let file = TmpFile::new(self.handle.clone(), inode_idx);
        Ok(FileHandle::new(meta, Box::new(file)))
    }

    fn metadata(&self, path: &str) -> FsResult<Metadata> {
        if path == "/" || path.is_empty() {
            return Ok(self.handle.root_metadata());
        }
        let inode_idx = self.handle.resolve_path(path)?;
        let inode = self.handle.read_inode(inode_idx)?;
        let name = path.rsplit('/').next().unwrap_or(path);
        let entry_type = if inode.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };
        let mut meta = Metadata::new(
            String::from(name),
            entry_type,
            inode.size() as usize,
            None,
            None,
            None,
        );
        meta.links = inode.links_count() as usize;
        Ok(meta)
    }

    fn exists(&self, path: &str) -> FsResult<bool> {
        match self.handle.resolve_path(path) {
            Ok(_) => Ok(true),
            Err(FsError::FileNotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn create_file(&self, path: &str) -> FsResult<FileHandle> {
        let parts: Vec<&str> = path
            .split(PATH_SEPARATOR)
            .filter(|p| !p.is_empty())
            .collect();

        if parts.is_empty() {
            return Err(FsError::InvalidPath(path.into()));
        }

        let file_name = parts[parts.len() - 1];
        let dir_path_parts = &parts[..parts.len() - 1];

        let mut dir_inode_idx = 0u16;
        for part in dir_path_parts {
            let inode = self.handle.read_inode(dir_inode_idx)?;
            if !inode.is_dir() {
                return Err(FsError::NotADirectory);
            }
            dir_inode_idx = self.handle.find_dir_entry(dir_inode_idx, part)?;
        }

        let new_inode_idx = self.handle.allocate_inode()?;
        let new_inode = InodeData::new_file();
        self.handle.write_inode(new_inode_idx, &new_inode)?;

        self.handle.add_dir_entry(dir_inode_idx, file_name, new_inode_idx)?;

        let meta = Metadata::new(
            String::from(file_name),
            FileType::File,
            0,
            None,
            None,
            None,
        );
        let file = TmpFile::new(self.handle.clone(), new_inode_idx);
        Ok(FileHandle::new(meta, Box::new(file)))
    }

    fn create_dir(&self, path: &str) -> FsResult {
        let parts: Vec<&str> = path
            .split(PATH_SEPARATOR)
            .filter(|p| !p.is_empty())
            .collect();

        if parts.is_empty() {
            return Err(FsError::InvalidPath(path.into()));
        }

        let dir_name = parts[parts.len() - 1];
        let parent_path_parts = &parts[..parts.len() - 1];

        let mut parent_inode_idx = 0u16;
        for part in parent_path_parts {
            let inode = self.handle.read_inode(parent_inode_idx)?;
            if !inode.is_dir() {
                return Err(FsError::NotADirectory);
            }
            parent_inode_idx = self.handle.find_dir_entry(parent_inode_idx, part)?;
        }

        let new_inode_idx = self.handle.allocate_inode()?;
        let new_inode = InodeData::new_dir();
        self.handle.write_inode(new_inode_idx, &new_inode)?;

        self.handle.add_dir_entry(parent_inode_idx, dir_name, new_inode_idx)?;

        Ok(())
    }

    fn link(&self, src: &str, dst: &str) -> FsResult {
        let src_inode_idx = self.handle.resolve_path(src)?;
        let src_inode = self.handle.read_inode(src_inode_idx)?;
        if !src_inode.is_file() {
            return Err(FsError::NotAFile);
        }

        let dst_parts: Vec<&str> = dst
            .split(PATH_SEPARATOR)
            .filter(|p| !p.is_empty())
            .collect();
        if dst_parts.is_empty() {
            return Err(FsError::InvalidPath(dst.into()));
        }

        let link_name = dst_parts[dst_parts.len() - 1];
        let dir_parts = &dst_parts[..dst_parts.len() - 1];

        let mut dir_inode_idx = 0u16;
        for part in dir_parts {
            let inode = self.handle.read_inode(dir_inode_idx)?;
            if !inode.is_dir() {
                return Err(FsError::NotADirectory);
            }
            dir_inode_idx = self.handle.find_dir_entry(dir_inode_idx, part)?;
        }

        self.handle.add_dir_entry(dir_inode_idx, link_name, src_inode_idx)?;

        let mut inode = self.handle.read_inode(src_inode_idx)?;
        inode.set_links_count(inode.links_count() + 1);
        self.handle.write_inode(src_inode_idx, &inode)?;

        Ok(())
    }
}