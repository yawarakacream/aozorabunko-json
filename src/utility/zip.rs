use std::io::{self, Read};

use anyhow::{Context, Result};
use zip::{read::ZipFile, ZipArchive};

pub struct ZipReader<R> {
    archive: ZipArchive<R>,
}

impl<R: Read + io::Seek> ZipReader<R> {
    pub fn new(reader: R) -> Result<ZipReader<R>> {
        let archive = ZipArchive::new(reader).context("Failed to open")?;
        Ok(ZipReader { archive })
    }

    pub fn len(self: &Self) -> usize {
        self.archive.len()
    }

    pub fn get_by_path(&mut self, path: &str) -> Result<ZipEntry> {
        self.archive
            .by_name(path)
            .with_context(|| format!("Failed to open {}", path))
            .map(|file| ZipEntry { file })
    }

    pub fn get_by_index(&mut self, index: usize) -> Result<ZipEntry> {
        self.archive
            .by_index(index)
            .with_context(|| format!("Failed to open at {}", index))
            .map(|file| ZipEntry { file })
    }
}

pub struct ZipEntry<'a> {
    file: ZipFile<'a>,
}

impl ZipEntry<'_> {
    pub fn name(self: &Self) -> &str {
        self.file.name()
    }

    pub fn as_bytes(self: &mut Self) -> Result<Vec<u8>> {
        let mut data = Vec::<u8>::new();
        self.file
            .read_to_end(&mut data)
            .with_context(|| format!("Failed to read {}", self.name()))?;

        Ok(data)
    }

    pub fn as_string(self: &mut Self) -> Result<String> {
        let mut data = String::new();
        self.file
            .read_to_string(&mut data)
            .with_context(|| format!("Failed to read {}", self.name()))?;

        Ok(data)
    }
}
