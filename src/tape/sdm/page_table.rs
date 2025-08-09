//! Smart page table for tracking page metadata and versions
//! 
//! Tracks where each page is stored, access patterns, and maintains
//! historical versions for time-travel functionality.

use std::collections::{BTreeMap, HashMap, VecDeque};
use crate::tape::sdm::backends::StorageLocation;

/// Page table tracking all pages in the system
#[derive(Debug)]
pub struct PageTable {
    /// Active page entries
    entries: BTreeMap<i64, PageEntry>,
    
    /// Historical versions of pages
    history: BTreeMap<i64, VecDeque<HistoricalPage>>,
    
    /// Named checkpoints
    checkpoints: HashMap<String, CheckpointInfo>,
    
    /// Global version counter
    current_version: u64,
    
    /// Configuration
    max_history_per_page: usize,
}

/// Information about a single page
#[derive(Debug, Clone)]
pub struct PageEntry {
    /// Page number
    pub page_num: i64,
    
    /// Current storage location
    pub location: StorageLocation,
    
    /// Version number (for COW and history)
    pub version: u64,
    
    /// Access statistics
    pub stats: AccessStats,
    
    /// Is this page dirty (modified)?
    pub dirty: bool,
    
    /// Is this page compressed?
    pub compressed: bool,
    
    /// Size in bytes (may differ if compressed)
    pub size: usize,
}

/// Historical version of a page
#[derive(Debug, Clone)]
pub struct HistoricalPage {
    /// Version when this page was active
    pub version: u64,
    
    /// Storage location of historical data
    pub location: StorageLocation,
    
    /// Timestamp when replaced
    pub replaced_at: u64,
    
    /// Size of historical data
    pub size: usize,
    
    /// Whether this version is compressed
    pub compressed: bool,
}

/// Page access statistics
#[derive(Debug, Clone, Default)]
pub struct AccessStats {
    /// Total number of reads
    pub read_count: u64,
    
    /// Total number of writes
    pub write_count: u64,
    
    /// Last access timestamp (nanoseconds)
    pub last_access: u64,
    
    /// Last write timestamp
    pub last_write: u64,
    
    /// Access frequency (accesses per second)
    pub frequency: f32,
    
    /// Detected access pattern
    pub pattern: AccessPattern,
}

/// Detected access patterns
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AccessPattern {
    #[default]
    Unknown,
    Sequential,
    Random,
    Temporal,  // Accessed during time-travel
    WriteOnce, // Written once, read many
}

/// Checkpoint information
#[derive(Debug, Clone)]
pub struct CheckpointInfo {
    /// Name of the checkpoint
    pub name: String,
    
    /// Version at checkpoint creation
    pub version: u64,
    
    /// Timestamp of creation
    pub created_at: u64,
    
    /// Pages modified since this checkpoint
    pub modified_pages: Vec<i64>,
}

impl PageTable {
    /// Create a new page table
    pub fn new() -> Self {
        PageTable {
            entries: BTreeMap::new(),
            history: BTreeMap::new(),
            checkpoints: HashMap::new(),
            current_version: 0,
            max_history_per_page: 10, // Keep last 10 versions
        }
    }
    
    /// Get a page entry
    pub fn get_page(&self, page_num: i64) -> Option<&PageEntry> {
        self.entries.get(&page_num)
    }
    
    /// Get a mutable page entry
    pub fn get_page_mut(&mut self, page_num: i64) -> Option<&mut PageEntry> {
        self.entries.get_mut(&page_num)
    }
    
    /// Get or create a page entry
    pub fn get_or_create_page(&mut self, page_num: i64) -> &mut PageEntry {
        let current_version = self.current_version;
        
        self.entries.entry(page_num).or_insert_with(|| {
            PageEntry {
                page_num,
                location: StorageLocation::Unallocated,
                version: current_version,
                stats: AccessStats::default(),
                dirty: false,
                compressed: false,
                size: 4096, // Default page size
            }
        })
    }
    
    /// Update page location and manage history
    pub fn update_page_location(&mut self, page_num: i64, new_location: StorageLocation) {
        // Clone the entry if it exists and needs to be saved to history
        let needs_history = if let Some(entry) = self.entries.get(&page_num) {
            entry.location != StorageLocation::Unallocated && entry.location != new_location
        } else {
            false
        };
        
        if needs_history {
            let entry = self.entries.get(&page_num).unwrap().clone();
            self.add_to_history(entry);
        }
        
        // Get next version before modifying entry
        let next_version = self.next_version();
        
        if let Some(entry) = self.entries.get_mut(&page_num) {
            entry.location = new_location;
            entry.version = next_version;
            entry.dirty = false;
        }
    }
    
    /// Mark a page as dirty
    pub fn mark_dirty(&mut self, page_num: i64) {
        if let Some(entry) = self.entries.get_mut(&page_num) {
            entry.dirty = true;
            entry.stats.last_write = current_timestamp();
            entry.stats.write_count += 1;
        }
    }
    
    /// Update access statistics
    pub fn record_access(&mut self, page_num: i64, is_write: bool) {
        if let Some(entry) = self.entries.get_mut(&page_num) {
            let now = current_timestamp();
            
            if is_write {
                entry.stats.write_count += 1;
                entry.stats.last_write = now;
            } else {
                entry.stats.read_count += 1;
            }
            
            // Update frequency (simple exponential moving average)
            let time_since_last = (now - entry.stats.last_access) as f32 / 1_000_000_000.0; // Convert to seconds
            if time_since_last > 0.0 {
                let instant_frequency = 1.0 / time_since_last;
                entry.stats.frequency = 0.9 * entry.stats.frequency + 0.1 * instant_frequency;
            }
            
            entry.stats.last_access = now;
        }
    }
    
    /// Create a checkpoint
    pub fn create_checkpoint(&mut self, name: String) {
        let checkpoint = CheckpointInfo {
            name: name.clone(),
            version: self.current_version,
            created_at: current_timestamp(),
            modified_pages: Vec::new(),
        };
        
        self.checkpoints.insert(name, checkpoint);
    }
    
    /// Get pages modified since a checkpoint
    pub fn get_modified_since_checkpoint(&self, checkpoint_name: &str) -> Option<Vec<i64>> {
        self.checkpoints.get(checkpoint_name).map(|cp| {
            self.entries
                .iter()
                .filter(|(_, entry)| entry.version > cp.version)
                .map(|(page_num, _)| *page_num)
                .collect()
        })
    }
    
    /// Read historical version of a page
    pub fn read_historical(&self, page_num: i64, target_version: u64) -> Option<Vec<u8>> {
        // Check if current version matches
        if let Some(entry) = self.entries.get(&page_num) {
            if entry.version <= target_version {
                // Current version is what we want
                // In real implementation, would read from storage
                return Some(vec![0u8; entry.size]);
            }
        }
        
        // Search in history
        if let Some(history) = self.history.get(&page_num) {
            for historical in history.iter().rev() {
                if historical.version <= target_version {
                    // Found the right historical version
                    // In real implementation, would read from storage
                    return Some(vec![0u8; historical.size]);
                }
            }
        }
        
        None
    }
    
    /// Add a page to history
    fn add_to_history(&mut self, entry: PageEntry) {
        let historical = HistoricalPage {
            version: entry.version,
            location: entry.location.clone(),
            replaced_at: current_timestamp(),
            size: entry.size,
            compressed: entry.compressed,
        };
        
        let history = self.history.entry(entry.page_num).or_insert_with(VecDeque::new);
        history.push_front(historical);
        
        // Limit history size
        while history.len() > self.max_history_per_page {
            history.pop_back();
        }
    }
    
    /// Get next version number
    fn next_version(&mut self) -> u64 {
        self.current_version += 1;
        self.current_version
    }
    
    /// Get pages that should be migrated based on access patterns
    pub fn suggest_migrations(&self, limit: usize) -> Vec<(i64, MigrationSuggestion)> {
        let mut suggestions = Vec::new();
        
        for (page_num, entry) in &self.entries {
            // Skip if already in optimal location
            if matches!(entry.location, StorageLocation::Unallocated) {
                continue;
            }
            
            let suggestion = self.analyze_page_for_migration(entry);
            if suggestion.is_some() {
                suggestions.push((*page_num, suggestion.unwrap()));
            }
        }
        
        // Sort by priority and take top N
        suggestions.sort_by(|a, b| b.1.priority.partial_cmp(&a.1.priority).unwrap());
        suggestions.truncate(limit);
        
        suggestions
    }
    
    /// Analyze a page to determine if it should be migrated
    fn analyze_page_for_migration(&self, entry: &PageEntry) -> Option<MigrationSuggestion> {
        let age = (current_timestamp() - entry.stats.last_access) as f32 / 1_000_000_000.0;
        
        // Hot page in cold storage?
        if entry.stats.frequency > 10.0 && matches!(entry.location, StorageLocation::Local { .. }) {
            return Some(MigrationSuggestion {
                target: StorageLocation::Dram { key: entry.page_num as u64 },
                reason: MigrationReason::HotData,
                priority: entry.stats.frequency,
            });
        }
        
        // Cold page in hot storage?
        if age > 300.0 && matches!(entry.location, StorageLocation::Dram { .. }) {
            return Some(MigrationSuggestion {
                target: StorageLocation::Local { file_id: 0, offset: 0 },
                reason: MigrationReason::ColdData,
                priority: 1.0 / age,
            });
        }
        
        None
    }
}

/// Suggestion for page migration
#[derive(Debug)]
pub struct MigrationSuggestion {
    /// Target storage location
    pub target: StorageLocation,
    
    /// Reason for migration
    pub reason: MigrationReason,
    
    /// Priority (higher = more important)
    pub priority: f32,
}

/// Reasons for page migration
#[derive(Debug, Clone, Copy)]
pub enum MigrationReason {
    HotData,      // Frequently accessed, move to faster storage
    ColdData,     // Rarely accessed, move to slower storage
    Checkpoint,   // Part of checkpoint, ensure durability
    Compression,  // Compress before moving to cold storage
}

impl PageEntry {
    /// Increment version number
    pub fn increment_version(&mut self) {
        self.version += 1;
    }
    
    /// Update access time
    pub fn update_access_time(&mut self) {
        self.stats.last_access = current_timestamp();
    }
}

/// Get current timestamp in nanoseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_page_creation() {
        let mut table = PageTable::new();
        
        let entry = table.get_or_create_page(0);
        assert_eq!(entry.page_num, 0);
        assert!(matches!(entry.location, StorageLocation::Unallocated));
    }
    
    #[test]
    fn test_access_tracking() {
        let mut table = PageTable::new();
        
        table.get_or_create_page(0);
        table.record_access(0, false); // Read
        table.record_access(0, true);  // Write
        
        let entry = table.get_page(0).unwrap();
        assert_eq!(entry.stats.read_count, 1);
        assert_eq!(entry.stats.write_count, 1);
    }
    
    #[test]
    fn test_checkpoint() {
        let mut table = PageTable::new();
        
        // Create pages and checkpoint
        table.get_or_create_page(0);
        table.get_or_create_page(1);
        table.create_checkpoint("test".to_string());
        
        // Modify one page
        table.mark_dirty(0);
        table.current_version += 1;
        table.get_page_mut(0).unwrap().version = table.current_version;
        
        // Check modified pages
        let modified = table.get_modified_since_checkpoint("test").unwrap();
        assert_eq!(modified.len(), 1);
        assert_eq!(modified[0], 0);
    }
}