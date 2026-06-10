use super::*;

impl Fat16Impl {
    pub fn new(inner: impl BlockDevice<Block512>) -> Self {
        let mut block = Block::default();

        inner.read_block(0, &mut block).unwrap();
        let bpb = Fat16Bpb::new(block.as_ref()).unwrap();

        trace!("Loading Fat16 Volume: {:#?}", bpb);

        // HINT: FirstDataSector = BPB_ResvdSecCnt + (BPB_NumFATs * FATSz) +
        // RootDirSectors;
        // FIXED: parse the bpb block to get basic infos
        let fat_start = bpb.reserved_sector_count() as usize;
        let bytes_per_sector = bpb.bytes_per_sector() as usize;
        let root_dir_sectors =
            (bpb.root_entries_count() as usize * DirEntry::LEN + bytes_per_sector - 1)
                / bytes_per_sector;
        let first_root_dir_sector =
            fat_start + bpb.fat_count() as usize * bpb.sectors_per_fat() as usize;
        let first_data_sector = first_root_dir_sector + root_dir_sectors;

        Self {
            bpb,
            inner: Box::new(inner),
            fat_start,
            first_data_sector,
            first_root_dir_sector,
        }
    }

    #[inline(always)]
    pub fn blocks_per_sector(&self) -> usize {
        self.bpb.bytes_per_sector() as usize / BLOCK_SIZE
    }

    #[inline(always)]
    pub fn sector_to_block(&self, sector: usize) -> usize {
        sector * self.blocks_per_sector()
    }

    pub fn cluster_to_sector(&self, cluster: &Cluster) -> usize {
        match *cluster {
            Cluster::ROOT_DIR => self.first_root_dir_sector,
            Cluster(c) => {
                // FIXED: calculate the first sector of the cluster
                // HINT: FirstSectorofCluster = ((N – 2) * BPB_SecPerClus) +
                // FirstDataSector;
                (((c - 2) * self.bpb.sectors_per_cluster() as u32) + self.first_data_sector as u32)
                    as usize
            }
        }
    }

    // File system helpers for FAT traversal, path lookup, and directory reads.
    pub fn next_cluster(&self, cluster: &Cluster) -> Cluster {
        match *cluster {
            Cluster::EMPTY => return Cluster::EMPTY,
            Cluster::ROOT_DIR | Cluster::END_OF_FILE => return Cluster::END_OF_FILE,
            Cluster::BAD => return Cluster::BAD,
            Cluster::INVALID => return Cluster::INVALID,
            Cluster(c) if c < 2 => return Cluster::INVALID,
            _ => {}
        }

        let fat_start_block = self.sector_to_block(self.fat_start);
        let fat_offset_by_bytes = cluster.0 as usize * 2;
        let block_offset = fat_offset_by_bytes / BLOCK_SIZE;
        let entry_offset = fat_offset_by_bytes % BLOCK_SIZE;

        let mut block = Block::default();
        self.inner
            .read_block(fat_start_block + block_offset, &mut block)
            .expect("failed to read FAT block");

        let fat_entry = u16::from_le_bytes(
            block.as_ref()[entry_offset..entry_offset + 2]
                .try_into()
                .unwrap(),
        );

        match fat_entry {
            0x0000 => Cluster::EMPTY,
            0x0001 | 0xFFF0..=0xFFF6 => Cluster::INVALID,
            0xFFF7 => Cluster::BAD,
            0xFFF8..=0xFFFF => Cluster::END_OF_FILE,
            next => Cluster(next as u32),
        }
    }

    pub fn get_direntry(&self, dir: &Directory, name: &str) -> FsResult<DirEntry> {
        let target = ShortFileName::parse(&name.to_ascii_uppercase())?;

        let search_sector = |sector: usize| -> FsResult<Option<DirEntry>> {
            let start_block = self.sector_to_block(sector);

            for block_offset in 0..self.blocks_per_sector() {
                let mut block = Block::default();
                self.inner
                    .read_block(start_block + block_offset, &mut block)?;

                for raw_entry in block.as_ref().chunks_exact(DirEntry::LEN) {
                    // FAT directory: first byte 0x00 means no more entries after this.
                    if raw_entry[0] == 0x00 {
                        return Err(FsError::FileNotFound);
                    }

                    let entry = DirEntry::parse(raw_entry)?;

                    if !entry.is_valid()
                        || entry.is_long_name()
                        || entry.attributes.contains(Attributes::VOLUME_ID)
                    {
                        continue;
                    }

                    if entry.filename.matches(&target) {
                        return Ok(Some(entry));
                    }
                }
            }

            Ok(None)
        };

        if dir.cluster == Cluster::ROOT_DIR {
            for sector in self.first_root_dir_sector..self.first_data_sector {
                if let Some(entry) = search_sector(sector)? {
                    return Ok(entry);
                }
            }

            return Err(FsError::FileNotFound);
        }

        let mut cluster = dir.cluster;

        loop {
            match cluster {
                Cluster::END_OF_FILE | Cluster::EMPTY => return Err(FsError::FileNotFound),
                Cluster::BAD => return Err(FsError::BadCluster),
                Cluster::INVALID | Cluster::ROOT_DIR => return Err(FsError::InvalidOperation),
                Cluster(c) if c < 2 => return Err(FsError::InvalidOperation),
                _ => {}
            }

            let first_sector = self.cluster_to_sector(&cluster);

            for offset in 0..self.bpb.sectors_per_cluster() as usize {
                if let Some(entry) = search_sector(first_sector + offset)? {
                    return Ok(entry);
                }
            }

            cluster = self.next_cluster(&cluster);
        }
    }

    fn root_metadata(&self) -> Metadata {
        Metadata::new(String::from("/"), FileType::Directory, 0, None, None, None)
    }

    fn is_entry_visible(entry: &DirEntry) -> bool {
        if !entry.is_valid()
            || entry.is_long_name()
            || entry.attributes.contains(Attributes::VOLUME_ID)
        {
            return false;
        }

        let name = entry.filename.basename().trim_end();
        name != "." && name != ".."
    }

    fn resolve_path(&self, path: &str) -> FsResult<Option<DirEntry>> {
        let mut parts = path
            .split(PATH_SEPARATOR)
            .filter(|part| !part.is_empty())
            .peekable();
        let mut dir = Directory::root();

        while let Some(part) = parts.next() {
            let entry = self.get_direntry(&dir, part)?;

            if parts.peek().is_none() {
                return Ok(Some(entry));
            }

            if !entry.is_directory() {
                return Err(FsError::NotADirectory);
            }

            dir = Directory::from_entry(entry);
        }

        Ok(None)
    }

    fn open_dir(&self, path: &str) -> FsResult<Directory> {
        match self.resolve_path(path)? {
            Some(entry) if entry.is_directory() => Ok(Directory::from_entry(entry)),
            Some(_) => Err(FsError::NotADirectory),
            None => Ok(Directory::root()),
        }
    }

    fn read_dir_sector(&self, sector: usize, entries: &mut Vec<Metadata>) -> FsResult<bool> {
        let start_block = self.sector_to_block(sector);

        for block_offset in 0..self.blocks_per_sector() {
            let mut block = Block::default();
            self.inner
                .read_block(start_block + block_offset, &mut block)?;

            for raw_entry in block.as_ref().chunks_exact(DirEntry::LEN) {
                if raw_entry[0] == 0x00 {
                    return Ok(true);
                }

                let entry = DirEntry::parse(raw_entry)?;

                if Self::is_entry_visible(&entry) {
                    entries.push(entry.as_meta());
                }
            }
        }

        Ok(false)
    }

    pub fn traverse_dir(&self, dir: &Directory) -> FsResult<Vec<Metadata>> {
        let mut entries = Vec::new();

        if dir.cluster == Cluster::ROOT_DIR {
            for sector in self.first_root_dir_sector..self.first_data_sector {
                if self.read_dir_sector(sector, &mut entries)? {
                    break;
                }
            }

            return Ok(entries);
        }

        let mut cluster = dir.cluster;

        loop {
            match cluster {
                Cluster::END_OF_FILE => return Ok(entries),
                Cluster::EMPTY => return Err(FsError::FileNotFound),
                Cluster::BAD => return Err(FsError::BadCluster),
                Cluster::INVALID | Cluster::ROOT_DIR => return Err(FsError::InvalidOperation),
                Cluster(c) if c < 2 => return Err(FsError::InvalidOperation),
                _ => {}
            }

            let first_sector = self.cluster_to_sector(&cluster);

            for offset in 0..self.bpb.sectors_per_cluster() as usize {
                if self.read_dir_sector(first_sector + offset, &mut entries)? {
                    return Ok(entries);
                }
            }

            cluster = self.next_cluster(&cluster);
        }
    }
}

impl FileSystem for Fat16 {
    fn read_dir(&self, path: &str) -> FsResult<Box<dyn Iterator<Item = Metadata> + Send>> {
        let dir = self.handle.open_dir(path)?;
        let entries = self.handle.traverse_dir(&dir)?;

        Ok(Box::new(entries.into_iter()))
    }

    fn open_file(&self, path: &str) -> FsResult<FileHandle> {
        match self.handle.resolve_path(path)? {
            Some(entry) if !entry.is_directory() => {
                let meta = entry.as_meta();
                let file = File::new(self.handle.clone(), entry);

                Ok(FileHandle::new(meta, Box::new(file)))
            }
            Some(_) | None => Err(FsError::NotAFile),
        }
    }

    fn metadata(&self, path: &str) -> FsResult<Metadata> {
        match self.handle.resolve_path(path)? {
            Some(entry) => Ok(entry.as_meta()),
            None => Ok(self.handle.root_metadata()),
        }
    }

    fn exists(&self, path: &str) -> FsResult<bool> {
        match self.handle.resolve_path(path) {
            Ok(_) => Ok(true),
            Err(FsError::FileNotFound | FsError::NotADirectory) => Ok(false),
            Err(err) => Err(err),
        }
    }
}
