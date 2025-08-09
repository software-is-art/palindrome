//! Segment management for structured data on tape

use super::core::{Tape, TrailOp};
use std::collections::HashMap;

/// A named region of tape
#[derive(Clone, Debug)]
pub struct Segment {
    pub name: String,
    pub start: i64,
    pub size: usize,
    pub segment_type: SegmentType,
    /// Index structures for this segment
    pub indices: Vec<Index>,
}

#[derive(Clone, Debug)]
pub enum SegmentType {
    Code,
    Data,
    Stack,
    Heap,
    Table { schema: Schema },
    Index,
    Log,
}

#[derive(Clone, Debug)]
pub struct Schema {
    pub fields: Vec<Field>,
    pub primary_key: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub dtype: DataType,
    pub nullable: bool,
}

#[derive(Clone, Debug)]
pub enum DataType {
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    String { max_len: Option<usize> },
    Bytes { max_len: Option<usize> },
    Timestamp,
}

/// Index structure for fast lookups
#[derive(Clone, Debug)]
pub struct Index {
    pub name: String,
    pub index_type: IndexType,
    pub fields: Vec<String>,
    /// B-tree nodes stored on tape
    pub root_position: i64,
}

#[derive(Clone, Debug)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
    FullText,
}

/// Extension trait to add segment functionality to Tape
pub trait SegmentExt {
    fn create_segment(
        &mut self, 
        name: String, 
        size: usize,
        segment_type: SegmentType
    ) -> Result<i64, String>;
    
    fn read_segment(
        &self, 
        name: &str, 
        offset: i64, 
        len: usize
    ) -> Result<Vec<u8>, String>;
    
    fn write_segment(
        &mut self,
        name: &str,
        offset: i64,
        data: &[u8]
    ) -> Result<(), String>;
    
    fn get_segment(&self, name: &str) -> Option<&Segment>;
    
    fn list_segments(&self) -> Vec<&Segment>;
}

/// Extended tape with segment support
#[derive(Clone)]
pub struct SegmentedTape {
    pub tape: Tape,
    pub segments: HashMap<String, Segment>,
}

impl SegmentedTape {
    pub fn new() -> Self {
        let tape = Tape::new();
        let segments = HashMap::new();
        
        SegmentedTape { tape, segments }
    }
    
    pub fn create_segment(
        &mut self, 
        name: String, 
        size: usize,
        segment_type: SegmentType
    ) -> Result<i64, String> {
        if self.segments.contains_key(&name) {
            return Err(format!("Segment '{}' already exists", name));
        }
        
        // Find free space (simple first-fit for now)
        let start = self.find_free_space(size)?;
        
        let segment = Segment {
            name: name.clone(),
            start,
            size,
            segment_type,
            indices: Vec::new(),
        };
        
        // Record segment creation in trail
        self.tape.add_trail_op(TrailOp::SegmentCreate {
            name: name.clone(),
            start,
            size,
        });
        
        self.segments.insert(name, segment);
        Ok(start)
    }
    
    pub fn read_segment(
        &self, 
        name: &str, 
        offset: i64, 
        len: usize
    ) -> Result<Vec<u8>, String> {
        let segment = self.segments.get(name)
            .ok_or_else(|| format!("Unknown segment: {}", name))?;
        
        if offset < 0 || offset + len as i64 > segment.size as i64 {
            return Err("Segment bounds violation".to_string());
        }
        
        let _old_pos = self.tape.position();
        let mut tape = self.tape.clone();
        tape.seek(segment.start + offset);
        let data = tape.read(len);
        
        Ok(data)
    }
    
    pub fn write_segment(
        &mut self,
        name: &str,
        offset: i64,
        data: &[u8]
    ) -> Result<(), String> {
        let segment = self.segments.get(name)
            .ok_or_else(|| format!("Unknown segment: {}", name))?
            .clone();
        
        if offset < 0 || offset + data.len() as i64 > segment.size as i64 {
            return Err("Segment bounds violation".to_string());
        }
        
        // Save current position
        let old_pos = self.tape.position();
        
        // Save old data for reversibility
        self.tape.seek(segment.start + offset);
        let old_data = self.tape.read(data.len());
        
        self.tape.add_trail_op(TrailOp::SegmentModify {
            name: name.to_string(),
            offset,
            old_data,
            new_data: data.to_vec(),
        });
        
        // Write new data
        self.tape.seek(segment.start + offset);
        self.tape.write(data);
        
        // Restore position
        self.tape.seek(old_pos);
        
        Ok(())
    }
    
    pub fn get_segment(&self, name: &str) -> Option<&Segment> {
        self.segments.get(name)
    }
    
    pub fn list_segments(&self) -> Vec<&Segment> {
        self.segments.values().collect()
    }
    
    fn find_free_space(&self, size: usize) -> Result<i64, String> {
        // Simple allocator: find gap between segments
        let mut segments: Vec<_> = self.segments.values()
            .map(|s| (s.start, s.start + s.size as i64))
            .collect();
        segments.sort_by_key(|s| s.0);
        
        let mut cursor = 0i64;
        for (start, end) in segments {
            if start - cursor >= size as i64 {
                return Ok(cursor);
            }
            cursor = end;
        }
        
        // Allocate at end
        Ok(cursor)
    }
}

impl Default for SegmentedTape {
    fn default() -> Self {
        Self::new()
    }
}

// Add segment operations to TrailOp
impl TrailOp {
    pub fn is_segment_op(&self) -> bool {
        matches!(self, 
            TrailOp::SegmentCreate { .. } | 
            TrailOp::SegmentModify { .. }
        )
    }
}

// Extend TrailOp enum in core.rs
#[derive(Clone, Debug)]
pub enum SegmentTrailOp {
    SegmentCreate {
        name: String,
        start: i64,
        size: usize,
    },
    SegmentModify {
        name: String,
        offset: i64,
        old_data: Vec<u8>,
        new_data: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_creation() {
        let mut stape = SegmentedTape::new();
        
        let start = stape.create_segment(
            "data".to_string(),
            1024,
            SegmentType::Data
        ).unwrap();
        
        assert_eq!(start, 0);
        assert!(stape.get_segment("data").is_some());
    }

    #[test]
    fn test_segment_read_write() {
        let mut stape = SegmentedTape::new();
        
        stape.create_segment(
            "data".to_string(),
            1024,
            SegmentType::Data
        ).unwrap();
        
        // Write to segment
        stape.write_segment("data", 0, b"Hello, World!").unwrap();
        
        // Read back
        let data = stape.read_segment("data", 0, 13).unwrap();
        assert_eq!(&data, b"Hello, World!");
    }

    #[test]
    fn test_segment_bounds_checking() {
        let mut stape = SegmentedTape::new();
        
        stape.create_segment(
            "small".to_string(),
            10,
            SegmentType::Data
        ).unwrap();
        
        // Should fail - out of bounds
        assert!(stape.write_segment("small", 5, b"too long").is_err());
        
        // Should succeed - within bounds
        assert!(stape.write_segment("small", 5, b"fits").is_ok());
    }

    #[test]
    fn test_multiple_segments() {
        let mut stape = SegmentedTape::new();
        
        let code_start = stape.create_segment(
            "code".to_string(),
            1024,
            SegmentType::Code
        ).unwrap();
        
        let data_start = stape.create_segment(
            "data".to_string(),
            2048,
            SegmentType::Data
        ).unwrap();
        
        let stack_start = stape.create_segment(
            "stack".to_string(),
            4096,
            SegmentType::Stack
        ).unwrap();
        
        // Segments should not overlap
        assert!(data_start >= code_start + 1024);
        assert!(stack_start >= data_start + 2048);
        
        assert_eq!(stape.list_segments().len(), 3);
    }

    #[test]
    fn test_segment_with_schema() {
        let mut stape = SegmentedTape::new();
        
        let schema = Schema {
            fields: vec![
                Field {
                    name: "id".to_string(),
                    dtype: DataType::Int64,
                    nullable: false,
                },
                Field {
                    name: "name".to_string(),
                    dtype: DataType::String { max_len: Some(64) },
                    nullable: false,
                },
            ],
            primary_key: vec!["id".to_string()],
        };
        
        stape.create_segment(
            "users".to_string(),
            65536,
            SegmentType::Table { schema }
        ).unwrap();
        
        // Verify segment exists with correct type
        if let Some(segment) = stape.get_segment("users") {
            matches!(segment.segment_type, SegmentType::Table { .. });
        } else {
            panic!("Segment not found");
        }
    }
}