//! # Simple Settings
//! This crate provides a very simple disk-based configuration storage.
//! It supports both saving new configuration and loading a new one.
//! Rust's type system ensures that all edits to the existing configuration are automatically saved on disk.

use {
    serde::{Deserialize, Serialize},
    std::{
        fs::{File, OpenOptions},
        io::{self, prelude::*},
        ops::{Deref, DerefMut},
        path::Path,
    },
};

/// A very simple TOML-based settings storage.
pub struct Settings<T> {
    file: std::fs::File,
    data: T,
}

/// Guard for read access.
pub struct SettingsGuard<'a, T> {
    data: &'a T,
}

impl<'a, T> Deref for SettingsGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// Guard for mutable access. Persists to disk upon destruction.
pub struct MutableSettingsGuard<'a, T>
where
    T: Serialize,
{
    data: &'a mut T,
    file: &'a mut std::fs::File,
}

impl<'a, T> Deref for MutableSettingsGuard<'a, T>
where
    T: Serialize,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> DerefMut for MutableSettingsGuard<'a, T>
where
    T: Serialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'a, T> Drop for MutableSettingsGuard<'a, T>
where
    T: Serialize,
{
    fn drop(&mut self) {
        self.file.set_len(0).unwrap();
        self.file.sync_all().unwrap();
        self.file.seek(std::io::SeekFrom::Start(0)).unwrap();
        self.file
            .write_all(&toml::to_vec(&self.data).unwrap())
            .unwrap();
        self.file.sync_all().unwrap();
    }
}

impl<T> Settings<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Create configuration and store it to disk.
    pub fn new(path: impl AsRef<Path>, data: T) -> io::Result<Self> {
        let mut s = Self {
            file: File::create(path)?,
            data,
        };
        let _ = s.guard_mut();
        Ok(s)
    }

    /// Load configuration from disk.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let path = path.as_ref().to_path_buf();
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map(Some)
            .unwrap_or_else(|_| None)
            .map(|mut file| {
                let mut s = String::new();
                file.read_to_string(&mut s)?;
                Ok(Self {
                    file,
                    data: toml::from_str(&s)?,
                })
            })
            .transpose()
    }

    /// Lock configuration for read access.
    pub fn guard(&self) -> SettingsGuard<T> {
        SettingsGuard { data: &self.data }
    }

    /// Lock configuration for mutable access. The created guard can be used for mutable access. Data will be saved on disk upon guard's destruction.
    pub fn guard_mut(&mut self) -> MutableSettingsGuard<T> {
        MutableSettingsGuard {
            data: &mut self.data,
            file: &mut self.file,
        }
    }
}
