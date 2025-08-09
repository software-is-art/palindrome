//! Storage backend implementations for SDM
//! 
//! Provides different storage tiers from fast DRAM to cold S3 storage,
//! all behind a unified interface.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use lru::LruCache;
use std::num::NonZeroUsize;

/// Trait for storage backends
pub trait StorageBackend: Send + Sync {
    /// Read data from the backend
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), String>;
    
    /// Write data to the backend
    fn write(&mut self, offset: u64, data: &[u8]) -> Result<(), String>;
    
    /// Get the latency of this backend in nanoseconds
    fn latency_ns(&self) -> u64;
    
    /// Get the bandwidth of this backend in MB/s
    fn bandwidth_mbps(&self) -> u64;
    
    /// Is this backend persistent across restarts?
    fn persistent(&self) -> bool;
    
    /// Get the name of this backend
    fn name(&self) -> &str;
}

/// Location of data in the storage hierarchy
#[derive(Debug, Clone, PartialEq)]
pub enum StorageLocation {
    /// In DRAM cache
    Dram { key: u64 },
    
    /// In local file
    Local { file_id: u32, offset: u64 },
    
    /// In network storage
    Network { node: String, offset: u64 },
    
    /// In cold storage (S3)
    Cold { key: String },
    
    /// Not yet allocated
    Unallocated,
}

/// Collection of all storage backends
pub struct StorageBackends {
    /// DRAM cache (fastest)
    pub dram: Arc<RwLock<MemoryBackend>>,
    
    /// Local SSD/disk storage
    pub local: Arc<RwLock<FileBackend>>,
    
    /// Network storage (future)
    pub network: Option<Arc<RwLock<NetworkBackend>>>,
    
    /// Cold storage (future)
    pub cold: Option<Arc<RwLock<S3Backend>>>,
}

/// In-memory storage backend using LRU cache
pub struct MemoryBackend {
    /// LRU cache mapping keys to data
    cache: LruCache<u64, Vec<u8>>,
    
    /// Total capacity in bytes
    capacity: usize,
    
    /// Current usage in bytes
    used: usize,
}

/// File-based storage backend
pub struct FileBackend {
    /// Base directory for storage files
    base_dir: PathBuf,
    
    /// Open file handles
    files: HashMap<u32, File>,
    
    /// Next file ID
    next_file_id: u32,
    
    /// File size limit
    file_size_limit: u64,
}

/// Network storage backend (placeholder)
pub struct NetworkBackend {
    // TODO: Implement network storage
}

/// S3 cold storage backend (placeholder)
pub struct S3Backend {
    // TODO: Implement S3 storage
    bucket: String,
    prefix: String,
}

impl StorageBackends {
    /// Create new storage backends with the given DRAM cache size
    pub fn new(dram_cache_size: usize) -> Self {
        StorageBackends {
            dram: Arc::new(RwLock::new(MemoryBackend::new(dram_cache_size))),
            local: Arc::new(RwLock::new(FileBackend::new("./palindrome_data"))),
            network: None,
            cold: None,
        }
    }
    
    /// Read from a storage location
    pub fn read(&self, location: &StorageLocation, size: usize) -> Result<Vec<u8>, String> {
        match location {
            StorageLocation::Dram { key } => {
                self.dram.read().unwrap().read_key(*key, size)
            }
            
            StorageLocation::Local { file_id, offset } => {
                let mut buf = vec![0u8; size];
                self.local.write().unwrap().read_from_file(*file_id, *offset, &mut buf)?;
                Ok(buf)
            }
            
            StorageLocation::Network { .. } => {
                Err("Network storage not implemented".to_string())
            }
            
            StorageLocation::Cold { .. } => {
                Err("Cold storage not implemented".to_string())
            }
            
            StorageLocation::Unallocated => {
                // Return zeros for unallocated pages
                Ok(vec![0u8; size])
            }
        }
    }
    
    /// Write to a storage location
    pub fn write(&mut self, location: &StorageLocation, data: &[u8]) -> Result<(), String> {
        match location {
            StorageLocation::Dram { key } => {
                self.dram.write().unwrap().write_key(*key, data)
            }
            
            StorageLocation::Local { file_id, offset } => {
                self.local.write().unwrap().write_to_file(*file_id, *offset, data)
            }
            
            StorageLocation::Network { .. } => {
                Err("Network storage not implemented".to_string())
            }
            
            StorageLocation::Cold { .. } => {
                Err("Cold storage not implemented".to_string())
            }
            
            StorageLocation::Unallocated => {
                Err("Cannot write to unallocated location".to_string())
            }
        }
    }
    
    /// Get the best backend for a given access pattern
    pub fn suggest_backend(&self, size: usize, access_frequency: f32) -> StorageLocation {
        // Simple policy: frequently accessed data goes to DRAM
        if access_frequency > 100.0 && size < 1024 * 1024 {
            // Try DRAM first
            let dram = self.dram.read().unwrap();
            if dram.available_space() >= size {
                return StorageLocation::Dram { key: rand::random() };
            }
        }
        
        // Otherwise use local storage
        StorageLocation::Local {
            file_id: 0, // Will be assigned by FileBackend
            offset: 0,
        }
    }
}

impl MemoryBackend {
    fn new(capacity: usize) -> Self {
        // Ensure at least 1 page in cache
        let num_pages = (capacity / 4096).max(1);
        let cache_size = NonZeroUsize::new(num_pages).unwrap();
        MemoryBackend {
            cache: LruCache::new(cache_size),
            capacity,
            used: 0,
        }
    }
    
    fn read_key(&self, key: u64, size: usize) -> Result<Vec<u8>, String> {
        if let Some(data) = self.cache.peek(&key) {
            if data.len() >= size {
                Ok(data[..size].to_vec())
            } else {
                Err(format!("Cached data too small: {} < {}", data.len(), size))
            }
        } else {
            Err("Key not found in DRAM cache".to_string())
        }
    }
    
    fn write_key(&mut self, key: u64, data: &[u8]) -> Result<(), String> {
        let data_vec = data.to_vec();
        let data_size = data_vec.len();
        
        // Check if we have space
        if self.used + data_size > self.capacity {
            // LRU eviction will happen automatically
            if let Some((_, evicted)) = self.cache.push(key, data_vec) {
                self.used -= evicted.len();
            }
        } else {
            self.cache.put(key, data_vec);
        }
        
        self.used += data_size;
        Ok(())
    }
    
    fn available_space(&self) -> usize {
        self.capacity.saturating_sub(self.used)
    }
}

impl StorageBackend for MemoryBackend {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), String> {
        // For DRAM backend, offset is the key
        let data = self.read_key(offset, buf.len())?;
        buf.copy_from_slice(&data);
        Ok(())
    }
    
    fn write(&mut self, offset: u64, data: &[u8]) -> Result<(), String> {
        self.write_key(offset, data)
    }
    
    fn latency_ns(&self) -> u64 {
        100 // 100ns for DRAM access
    }
    
    fn bandwidth_mbps(&self) -> u64 {
        25_600 // 25.6 GB/s typical DRAM bandwidth
    }
    
    fn persistent(&self) -> bool {
        false // DRAM is volatile
    }
    
    fn name(&self) -> &str {
        "DRAM"
    }
}

impl FileBackend {
    fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        let base_dir = base_dir.as_ref().to_path_buf();
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&base_dir).ok();
        
        FileBackend {
            base_dir,
            files: HashMap::new(),
            next_file_id: 0,
            file_size_limit: 1024 * 1024 * 1024, // 1GB per file
        }
    }
    
    fn get_or_create_file(&mut self, file_id: u32) -> Result<&mut File, String> {
        if !self.files.contains_key(&file_id) {
            let file_path = self.base_dir.join(format!("tape_{:08}.dat", file_id));
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&file_path)
                .map_err(|e| format!("Failed to open file: {}", e))?;
            
            self.files.insert(file_id, file);
        }
        
        Ok(self.files.get_mut(&file_id).unwrap())
    }
    
    fn read_from_file(&mut self, file_id: u32, offset: u64, buf: &mut [u8]) -> Result<(), String> {
        let file = self.get_or_create_file(file_id)?;
        
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Seek failed: {}", e))?;
        
        file.read_exact(buf)
            .map_err(|e| format!("Read failed: {}", e))?;
        
        Ok(())
    }
    
    fn write_to_file(&mut self, file_id: u32, offset: u64, data: &[u8]) -> Result<(), String> {
        let file = self.get_or_create_file(file_id)?;
        
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Seek failed: {}", e))?;
        
        file.write_all(data)
            .map_err(|e| format!("Write failed: {}", e))?;
        
        file.flush()
            .map_err(|e| format!("Flush failed: {}", e))?;
        
        Ok(())
    }
    
    pub fn allocate_space(&mut self, size: u64) -> Result<(u32, u64), String> {
        // Simple allocation: append to current file
        let file_id = self.next_file_id;
        let file = self.get_or_create_file(file_id)?;
        
        let offset = file.seek(SeekFrom::End(0))
            .map_err(|e| format!("Seek failed: {}", e))?;
        
        // Check if we need a new file
        if offset + size > self.file_size_limit {
            self.next_file_id += 1;
            return self.allocate_space(size);
        }
        
        Ok((file_id, offset))
    }
}

impl StorageBackend for FileBackend {
    fn read(&self, _offset: u64, _buf: &mut [u8]) -> Result<(), String> {
        Err("FileBackend requires file_id, use read_from_file".to_string())
    }
    
    fn write(&mut self, _offset: u64, _data: &[u8]) -> Result<(), String> {
        Err("FileBackend requires file_id, use write_to_file".to_string())
    }
    
    fn latency_ns(&self) -> u64 {
        100_000 // 100μs for SSD access
    }
    
    fn bandwidth_mbps(&self) -> u64 {
        500 // 500 MB/s typical SSD bandwidth
    }
    
    fn persistent(&self) -> bool {
        true // Files persist across restarts
    }
    
    fn name(&self) -> &str {
        "LocalFile"
    }
}

// Placeholder for random number generation
mod rand {
    pub fn random() -> u64 {
        // In real implementation, use proper RNG
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_backend() {
        let mut backend = MemoryBackend::new(1024);
        
        // Write some data
        backend.write_key(1, b"Hello").unwrap();
        
        // Read it back
        let data = backend.read_key(1, 5).unwrap();
        assert_eq!(&data, b"Hello");
    }
    
    #[test]
    fn test_file_backend() {
        let mut backend = FileBackend::new("./test_data");
        
        // Write some data
        backend.write_to_file(0, 0, b"Test data").unwrap();
        
        // Read it back
        let mut buf = vec![0u8; 9];
        backend.read_from_file(0, 0, &mut buf).unwrap();
        assert_eq!(&buf, b"Test data");
        
        // Cleanup
        std::fs::remove_dir_all("./test_data").ok();
    }
}