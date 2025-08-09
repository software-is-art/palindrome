//! Virtual address space management for SDM
//! 
//! Provides a unified view of the tape address space without
//! allocating actual memory. All addresses are virtual until accessed.

use std::collections::{BTreeMap, HashMap};

/// Virtual address space - no actual memory allocated
#[derive(Debug)]
pub struct VirtualAddressSpace {
    /// Defined regions with policy hints
    regions: BTreeMap<i64, Region>,
    
    /// Named marks for quick navigation
    marks: HashMap<String, i64>,
    
    /// Page size for alignment
    page_size: usize,
}

/// A region of virtual address space with metadata
#[derive(Debug, Clone)]
pub struct Region {
    /// Starting position of the region
    pub start: i64,
    
    /// Size of the region in bytes
    pub size: usize,
    
    /// Hint for memory placement policy
    pub hint: PolicyHint,
    
    /// Optional name for the region
    pub name: Option<String>,
}

/// Hints to guide memory placement policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyHint {
    /// Code segment - keep in fastest memory
    Code,
    
    /// Stack - high locality, frequent access
    Stack,
    
    /// Heap - variable access patterns
    Heap,
    
    /// Sequential data - good for prefetching
    Sequential,
    
    /// Random access data
    Random,
    
    /// Historical data - append-only, compress when cold
    History,
    
    /// Checkpoint data - durability matters
    Checkpoint,
    
    /// Temporary data - can be discarded
    Temporary,
    
    /// User-defined hint
    Custom(u32),
}

impl VirtualAddressSpace {
    /// Create a new virtual address space
    pub fn new(page_size: usize) -> Self {
        VirtualAddressSpace {
            regions: BTreeMap::new(),
            marks: HashMap::new(),
            page_size,
        }
    }
    
    /// Define a new region in the address space
    pub fn define_region(&mut self, start: i64, size: usize, hint: PolicyHint, name: Option<String>) -> Result<(), String> {
        // Check for overlaps
        if self.check_overlap(start, size) {
            return Err("Region overlaps with existing region".to_string());
        }
        
        let region = Region {
            start,
            size,
            hint,
            name: name.clone(),
        };
        
        self.regions.insert(start, region);
        
        // Auto-create mark for named regions
        if let Some(name) = name {
            self.marks.insert(name, start);
        }
        
        Ok(())
    }
    
    /// Get the policy hint for a given address
    pub fn get_hint(&self, address: i64) -> PolicyHint {
        // Find the region containing this address
        let mut hint = PolicyHint::Random; // Default
        
        for (_, region) in self.regions.range(..=address).rev() {
            if address >= region.start && address < region.start + region.size as i64 {
                hint = region.hint;
                break;
            }
        }
        
        hint
    }
    
    /// Get region information for an address
    pub fn get_region(&self, address: i64) -> Option<&Region> {
        for (_, region) in self.regions.range(..=address).rev() {
            if address >= region.start && address < region.start + region.size as i64 {
                return Some(region);
            }
        }
        None
    }
    
    /// Mark a position for quick seeking
    pub fn mark(&mut self, label: String, position: i64) {
        self.marks.insert(label, position);
    }
    
    /// Get a marked position
    pub fn get_mark(&self, label: &str) -> Option<i64> {
        self.marks.get(label).copied()
    }
    
    /// Check if a new region would overlap with existing ones
    fn check_overlap(&self, start: i64, size: usize) -> bool {
        let end = start + size as i64;
        
        for (_, region) in &self.regions {
            let region_end = region.start + region.size as i64;
            
            // Check for overlap
            if start < region_end && end > region.start {
                return true;
            }
        }
        
        false
    }
    
    /// Get all regions in a given range
    pub fn get_regions_in_range(&self, start: i64, end: i64) -> Vec<&Region> {
        let mut result = Vec::new();
        
        for (_, region) in &self.regions {
            let region_end = region.start + region.size as i64;
            
            // Check if region intersects with range
            if region.start < end && region_end > start {
                result.push(region);
            }
        }
        
        result
    }
    
    /// Calculate aligned page boundaries for an address range
    pub fn page_range(&self, start: i64, len: usize) -> (i64, i64) {
        let start_page = start / self.page_size as i64;
        let end_page = (start + len as i64 - 1) / self.page_size as i64;
        (start_page, end_page)
    }
    
    /// Get page-aligned address
    pub fn page_align(&self, address: i64) -> i64 {
        (address / self.page_size as i64) * self.page_size as i64
    }
}

impl PolicyHint {
    /// Get a descriptive name for the hint
    pub fn name(&self) -> &'static str {
        match self {
            PolicyHint::Code => "code",
            PolicyHint::Stack => "stack",
            PolicyHint::Heap => "heap",
            PolicyHint::Sequential => "sequential",
            PolicyHint::Random => "random",
            PolicyHint::History => "history",
            PolicyHint::Checkpoint => "checkpoint",
            PolicyHint::Temporary => "temporary",
            PolicyHint::Custom(_) => "custom",
        }
    }
    
    /// Suggest caching priority (higher = keep in faster memory)
    pub fn cache_priority(&self) -> u32 {
        match self {
            PolicyHint::Code => 100,       // Highest priority
            PolicyHint::Stack => 90,       // Very high
            PolicyHint::Checkpoint => 80,  // High (for quick rewind)
            PolicyHint::Sequential => 70,  // Good for prefetch
            PolicyHint::Heap => 50,        // Medium
            PolicyHint::Random => 40,      // Lower
            PolicyHint::History => 20,     // Can be cold
            PolicyHint::Temporary => 10,   // Lowest
            PolicyHint::Custom(p) => *p,   // User-defined
        }
    }
    
    /// Should this data be compressed when cold?
    pub fn should_compress(&self) -> bool {
        match self {
            PolicyHint::History => true,      // Historical data compresses well
            PolicyHint::Checkpoint => true,   // Checkpoints can be large
            PolicyHint::Code => false,        // Keep code uncompressed
            PolicyHint::Stack => false,       // Stack needs fast access
            _ => true,                        // Default: compress when cold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_region_definition() {
        let mut space = VirtualAddressSpace::new(4096);
        
        // Define some regions
        space.define_region(0, 1024 * 1024, PolicyHint::Code, Some("code".to_string())).unwrap();
        space.define_region(1024 * 1024, 1024 * 1024, PolicyHint::Stack, Some("stack".to_string())).unwrap();
        
        // Check marks were created
        assert_eq!(space.get_mark("code"), Some(0));
        assert_eq!(space.get_mark("stack"), Some(1024 * 1024));
    }
    
    #[test]
    fn test_overlap_detection() {
        let mut space = VirtualAddressSpace::new(4096);
        
        // Define a region
        space.define_region(1000, 1000, PolicyHint::Heap, None).unwrap();
        
        // Try to create overlapping region
        let result = space.define_region(1500, 1000, PolicyHint::Heap, None);
        assert!(result.is_err());
        
        // Non-overlapping should work
        let result = space.define_region(3000, 1000, PolicyHint::Heap, None);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_hint_lookup() {
        let mut space = VirtualAddressSpace::new(4096);
        
        space.define_region(0, 1000, PolicyHint::Code, None).unwrap();
        space.define_region(1000, 1000, PolicyHint::Stack, None).unwrap();
        
        assert_eq!(space.get_hint(500), PolicyHint::Code);
        assert_eq!(space.get_hint(1500), PolicyHint::Stack);
        assert_eq!(space.get_hint(5000), PolicyHint::Random); // Default
    }
    
    #[test]
    fn test_page_alignment() {
        let space = VirtualAddressSpace::new(4096);
        
        assert_eq!(space.page_align(0), 0);
        assert_eq!(space.page_align(100), 0);
        assert_eq!(space.page_align(4096), 4096);
        assert_eq!(space.page_align(4097), 4096);
        
        let (start, end) = space.page_range(100, 8000);
        assert_eq!(start, 0);  // Page 0
        assert_eq!(end, 1);    // Page 1 (covers bytes 100-8099)
    }
}