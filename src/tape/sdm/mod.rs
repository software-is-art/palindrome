//! Software Defined Memory (SDM) implementation for Palindrome VM
//! 
//! Provides a unified, policy-driven address space across all storage tiers
//! without OS complexity. The tape becomes truly infinite with automatic
//! management of data placement based on access patterns.

pub mod address_space;
pub mod backends;
pub mod page_table;
pub mod policy;
pub mod predictor;

use std::sync::{Arc, RwLock};

pub use address_space::{VirtualAddressSpace, Region, PolicyHint};
pub use backends::{StorageBackend, StorageBackends, StorageLocation};
pub use page_table::{PageTable, PageEntry};
pub use policy::{MemoryPolicy, PlacementRule};
pub use predictor::AccessPredictor;

/// The main SDM tape implementation
pub struct SdmTape {
    /// Virtual address space (no actual memory allocated)
    address_space: Arc<RwLock<VirtualAddressSpace>>,
    
    /// Memory placement policy
    policy: Arc<MemoryPolicy>,
    
    /// Physical storage backends
    backends: Arc<RwLock<StorageBackends>>,
    
    /// Page metadata tracking
    page_table: Arc<RwLock<PageTable>>,
    
    /// Access pattern learning and prediction
    predictor: Arc<RwLock<AccessPredictor>>,
    
    /// Configuration
    config: SdmConfig,
}

/// SDM configuration
#[derive(Clone, Debug)]
pub struct SdmConfig {
    /// Page size in bytes (default 4KB)
    pub page_size: usize,
    
    /// DRAM cache size in bytes
    pub dram_cache_size: usize,
    
    /// Prefetch depth for sequential access
    pub prefetch_depth: usize,
    
    /// Enable compression for cold data
    pub enable_compression: bool,
    
    /// Compression threshold in bytes
    pub compression_threshold: usize,
}

impl Default for SdmConfig {
    fn default() -> Self {
        SdmConfig {
            page_size: 4096,                    // 4KB pages
            dram_cache_size: 100 * 1024 * 1024, // 100MB DRAM cache
            prefetch_depth: 5,                  // Prefetch 5 pages ahead
            enable_compression: true,
            compression_threshold: 64 * 1024,   // Compress pages > 64KB
        }
    }
}

impl SdmTape {
    /// Create a new SDM tape with default configuration
    pub fn new() -> Self {
        Self::with_config(SdmConfig::default())
    }
    
    /// Create a new SDM tape with custom configuration
    pub fn with_config(config: SdmConfig) -> Self {
        SdmTape {
            address_space: Arc::new(RwLock::new(VirtualAddressSpace::new(config.page_size))),
            policy: Arc::new(MemoryPolicy::default()),
            backends: Arc::new(RwLock::new(StorageBackends::new(config.dram_cache_size))),
            page_table: Arc::new(RwLock::new(PageTable::new())),
            predictor: Arc::new(RwLock::new(AccessPredictor::new())),
            config,
        }
    }
    
    /// Read data from the tape at the given position
    pub fn read(&self, pos: i64, len: usize) -> Result<Vec<u8>, String> {
        // Record access for prediction
        self.predictor.write().unwrap().record_access(pos, len, false);
        
        // Calculate page range
        let start_page = pos / self.config.page_size as i64;
        let end_page = (pos + len as i64 - 1) / self.config.page_size as i64;
        
        let mut result = Vec::with_capacity(len);
        let page_table = self.page_table.read().unwrap();
        let backends = self.backends.read().unwrap();
        
        // Read each page
        for page_num in start_page..=end_page {
            let page_data = self.read_page(&page_table, &backends, page_num)?;
            
            // Calculate offsets within the page
            let page_start = page_num * self.config.page_size as i64;
            let offset_in_page = if page_num == start_page {
                (pos - page_start) as usize
            } else {
                0
            };
            
            let bytes_from_page = if page_num == end_page {
                let end_offset = ((pos + len as i64) - page_start) as usize;
                end_offset - offset_in_page
            } else {
                self.config.page_size - offset_in_page
            };
            
            result.extend_from_slice(&page_data[offset_in_page..offset_in_page + bytes_from_page]);
        }
        
        // Trigger prefetch if sequential access detected
        if let Some(prefetch_pages) = self.predictor.read().unwrap().suggest_prefetch(end_page) {
            self.prefetch_pages(prefetch_pages);
        }
        
        Ok(result)
    }
    
    /// Write data to the tape at the given position
    pub fn write(&self, pos: i64, data: &[u8]) -> Result<(), String> {
        // Record access for prediction
        self.predictor.write().unwrap().record_access(pos, data.len(), true);
        
        // Calculate page range
        let start_page = pos / self.config.page_size as i64;
        let end_page = (pos + data.len() as i64 - 1) / self.config.page_size as i64;
        
        let mut offset = 0;
        
        // Write each page
        for page_num in start_page..=end_page {
            let page_start = page_num * self.config.page_size as i64;
            let offset_in_page = if page_num == start_page {
                (pos - page_start) as usize
            } else {
                0
            };
            
            let bytes_to_write = if page_num == end_page {
                data.len() - offset
            } else {
                self.config.page_size - offset_in_page
            };
            
            self.write_page(page_num, offset_in_page, &data[offset..offset + bytes_to_write])?;
            offset += bytes_to_write;
        }
        
        Ok(())
    }
    
    /// Write data with instruction counter
    pub fn write_with_ic(&self, pos: i64, data: &[u8], ic: u64) -> Result<(), String> {
        // Record access for prediction
        self.predictor.write().unwrap().record_access(pos, data.len(), true);
        
        // Calculate page range
        let start_page = pos / self.config.page_size as i64;
        let end_page = (pos + data.len() as i64 - 1) / self.config.page_size as i64;
        
        let mut offset = 0;
        
        // Write each page
        for page_num in start_page..=end_page {
            let page_start = page_num * self.config.page_size as i64;
            let offset_in_page = if page_num == start_page {
                (pos - page_start) as usize
            } else {
                0
            };
            
            let bytes_to_write = if page_num == end_page {
                data.len() - offset
            } else {
                self.config.page_size - offset_in_page
            };
            
            self.write_page_with_ic(page_num, offset_in_page, &data[offset..offset + bytes_to_write], ic)?;
            offset += bytes_to_write;
        }
        
        Ok(())
    }
    
    /// Read data at a specific instruction counter
    pub fn read_at_ic(&self, pos: i64, len: usize, ic: u64) -> Result<Vec<u8>, String> {
        // Find the version of pages at the given IC
        let page_table = self.page_table.read().unwrap();
        let start_page = pos / self.config.page_size as i64;
        let end_page = (pos + len as i64 - 1) / self.config.page_size as i64;
        
        let mut result = Vec::with_capacity(len);
        
        for page_num in start_page..=end_page {
            // Get version at IC
            let page_data = page_table.read_at_ic(page_num, ic)
                .ok_or_else(|| format!("No data for page {} at IC {}", page_num, ic))?;
            
            // Calculate offsets (same as regular read)
            let page_start = page_num * self.config.page_size as i64;
            let offset_in_page = if page_num == start_page {
                (pos - page_start) as usize
            } else {
                0
            };
            
            let bytes_from_page = if page_num == end_page {
                let end_offset = ((pos + len as i64) - page_start) as usize;
                end_offset - offset_in_page
            } else {
                self.config.page_size - offset_in_page
            };
            
            result.extend_from_slice(&page_data[offset_in_page..offset_in_page + bytes_from_page]);
        }
        
        Ok(result)
    }
    
    /// Read data from a specific point in time
    pub fn read_at_time(&self, pos: i64, len: usize, timestamp: u64) -> Result<Vec<u8>, String> {
        // Find the version of pages at the given timestamp
        let page_table = self.page_table.read().unwrap();
        let start_page = pos / self.config.page_size as i64;
        let end_page = (pos + len as i64 - 1) / self.config.page_size as i64;
        
        let mut result = Vec::with_capacity(len);
        
        for page_num in start_page..=end_page {
            // Get historical version of the page
            let page_data = page_table.read_historical(page_num, timestamp)
                .ok_or_else(|| format!("No historical data for page {} at time {}", page_num, timestamp))?;
            
            // Calculate offsets (same as regular read)
            let page_start = page_num * self.config.page_size as i64;
            let offset_in_page = if page_num == start_page {
                (pos - page_start) as usize
            } else {
                0
            };
            
            let bytes_from_page = if page_num == end_page {
                let end_offset = ((pos + len as i64) - page_start) as usize;
                end_offset - offset_in_page
            } else {
                self.config.page_size - offset_in_page
            };
            
            result.extend_from_slice(&page_data[offset_in_page..offset_in_page + bytes_from_page]);
        }
        
        Ok(result)
    }
    
    /// Mark current position for quick seeking
    pub fn mark(&self, label: String, position: i64) -> Result<(), String> {
        self.address_space.write().unwrap().mark(label, position);
        Ok(())
    }
    
    /// Create a checkpoint of current state
    pub fn checkpoint(&self, name: String) -> Result<(), String> {
        self.page_table.write().unwrap().create_checkpoint(name);
        Ok(())
    }
    
    /// Internal: Read a single page
    fn read_page(&self, page_table: &PageTable, backends: &StorageBackends, page_num: i64) -> Result<Vec<u8>, String> {
        if let Some(entry) = page_table.get_page(page_num) {
            backends.read(&entry.location, self.config.page_size)
        } else {
            // Page doesn't exist, return zeros
            Ok(vec![0u8; self.config.page_size])
        }
    }
    
    /// Internal: Write to a page
    fn write_page(&self, page_num: i64, offset: usize, data: &[u8]) -> Result<(), String> {
        let mut page_table = self.page_table.write().unwrap();
        let mut backends = self.backends.write().unwrap();
        
        // Get or create page entry
        let entry = page_table.get_or_create_page(page_num);
        
        // Read existing page data if partial write or page already exists
        let mut page_data = if entry.location != StorageLocation::Unallocated && (offset > 0 || data.len() < self.config.page_size) {
            backends.read(&entry.location, self.config.page_size)?
        } else {
            vec![0u8; self.config.page_size]
        };
        
        // Update page data
        page_data[offset..offset + data.len()].copy_from_slice(data);
        
        // Allocate storage if needed
        let location = if entry.location == StorageLocation::Unallocated {
            // Allocate new storage - for now, always use DRAM with page number as key
            StorageLocation::Dram { key: page_num as u64 }
        } else {
            entry.location.clone()
        };
        
        // Write to location
        backends.write(&location, &page_data)?;
        
        // Update page table
        entry.location = location;
        entry.increment_version();
        entry.update_access_time();
        
        Ok(())
    }
    
    /// Internal: Write to a page with instruction counter
    fn write_page_with_ic(&self, page_num: i64, offset: usize, data: &[u8], ic: u64) -> Result<(), String> {
        let mut page_table = self.page_table.write().unwrap();
        let mut backends = self.backends.write().unwrap();
        
        // Record write with IC
        page_table.record_write_with_ic(page_num, ic);
        
        // Get the updated entry
        let entry = page_table.get_or_create_page(page_num);
        
        // Read existing page data if partial write or page already exists
        let mut page_data = if entry.location != StorageLocation::Unallocated && (offset > 0 || data.len() < self.config.page_size) {
            backends.read(&entry.location, self.config.page_size)?
        } else {
            vec![0u8; self.config.page_size]
        };
        
        // Update page data
        page_data[offset..offset + data.len()].copy_from_slice(data);
        
        // Allocate storage if needed
        let location = if entry.location == StorageLocation::Unallocated {
            // Allocate new storage - for now, always use DRAM with page number as key
            StorageLocation::Dram { key: page_num as u64 }
        } else {
            entry.location.clone()
        };
        
        // Write to location
        backends.write(&location, &page_data)?;
        
        // Update location
        entry.location = location;
        
        Ok(())
    }
    
    /// Prefetch pages based on access prediction
    fn prefetch_pages(&self, _pages: Vec<i64>) {
        // TODO: Implement async prefetching
        // This would spawn a background task to load pages into DRAM
    }
}

impl Default for SdmTape {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sdm_basic_read_write() {
        let tape = SdmTape::new();
        
        // Write some data
        tape.write(0, b"Hello, SDM!").unwrap();
        
        // Read it back
        let data = tape.read(0, 11).unwrap();
        assert_eq!(&data, b"Hello, SDM!");
    }
    
    #[test]
    fn test_sdm_cross_page_access() {
        let config = SdmConfig {
            page_size: 10, // Small pages for testing
            ..Default::default()
        };
        let tape = SdmTape::with_config(config);
        
        // Write across page boundary
        tape.write(8, b"Hello").unwrap(); // Spans pages 0 and 1
        
        // Read it back
        let data = tape.read(8, 5).unwrap();
        assert_eq!(&data, b"Hello");
    }
}