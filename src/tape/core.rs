//! Core tape implementation with reversibility support

use std::collections::{HashMap, BTreeMap};

/// The fundamental infinite tape abstraction
#[derive(Clone)]
pub struct Tape {
    /// Tape data organized in pages for efficiency
    pages: BTreeMap<i64, Page>,
    /// Current head position
    head: i64,
    /// Named marks for quick seeking
    marks: HashMap<String, i64>,
    /// History trail for reversibility
    trail: Trail,
}

/// A 4KB page of tape data
#[derive(Clone)]
pub struct Page {
    pub data: Box<[u8; 4096]>,
    /// Copy-on-write reference count
    pub cow_refs: usize,
}

/// History trail for reversibility
#[derive(Clone)]
pub struct Trail {
    /// Operations that can be undone
    pub operations: Vec<TrailOp>,
    /// Checkpoints for quick rewind
    pub checkpoints: HashMap<String, usize>,
}

#[derive(Clone, Debug)]
pub enum TrailOp {
    Write { 
        pos: i64, 
        old: Vec<u8>, 
        new: Vec<u8> 
    },
    Seek { 
        old_pos: i64, 
        new_pos: i64 
    },
    Mark {
        label: String,
        pos: i64,
    },
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

impl Tape {
    pub fn new() -> Self {
        Tape {
            pages: BTreeMap::new(),
            head: 0,
            marks: HashMap::new(),
            trail: Trail::new(),
        }
    }

    /// Read bytes at current position
    pub fn read(&self, len: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(len);
        let mut pos = self.head;
        
        while result.len() < len {
            let page_idx = pos / 4096;
            let page_offset = (pos % 4096) as usize;
            
            if let Some(page) = self.pages.get(&page_idx) {
                let available = (4096 - page_offset).min(len - result.len());
                result.extend_from_slice(
                    &page.data[page_offset..page_offset + available]
                );
                pos += available as i64;
            } else {
                // Uninitialized tape reads as zeros
                let zeros_needed = (len - result.len()).min(4096);
                result.resize(result.len() + zeros_needed, 0);
                pos += zeros_needed as i64;
            }
        }
        
        result
    }

    /// Write bytes at current position with COW
    pub fn write(&mut self, data: &[u8]) {
        let old_data = self.read(data.len());
        
        // Record for reversibility
        self.trail.operations.push(TrailOp::Write {
            pos: self.head,
            old: old_data,
            new: data.to_vec(),
        });
        
        let mut pos = self.head;
        let mut written = 0;
        
        while written < data.len() {
            let page_idx = pos / 4096;
            let page_offset = (pos % 4096) as usize;
            let to_write = (data.len() - written).min(4096 - page_offset);
            
            // Copy-on-write logic
            let page = self.pages.entry(page_idx).or_insert_with(|| {
                Page {
                    data: Box::new([0; 4096]),
                    cow_refs: 0,
                }
            });
            
            if page.cow_refs > 0 {
                // Need to copy before writing
                let mut new_data = page.data.clone();
                new_data[page_offset..page_offset + to_write]
                    .copy_from_slice(&data[written..written + to_write]);
                *page = Page {
                    data: new_data,
                    cow_refs: 0,
                };
            } else {
                page.data[page_offset..page_offset + to_write]
                    .copy_from_slice(&data[written..written + to_write]);
            }
            
            written += to_write;
            pos += to_write as i64;
        }
    }

    /// Seek to position
    pub fn seek(&mut self, pos: i64) {
        self.trail.operations.push(TrailOp::Seek {
            old_pos: self.head,
            new_pos: pos,
        });
        self.head = pos;
    }

    /// Move head by delta
    pub fn advance(&mut self, delta: i64) {
        self.seek(self.head + delta);
    }

    /// Mark current position with a label
    pub fn mark(&mut self, label: String) {
        self.trail.operations.push(TrailOp::Mark {
            label: label.clone(),
            pos: self.head,
        });
        self.marks.insert(label, self.head);
    }

    /// Seek to a marked position
    pub fn seek_mark(&mut self, label: &str) -> Result<(), String> {
        let pos = self.marks.get(label)
            .copied()
            .ok_or_else(|| format!("Unknown mark: {}", label))?;
        self.seek(pos);
        Ok(())
    }

    /// Create a checkpoint
    pub fn checkpoint(&mut self, name: String) {
        self.trail.checkpoints.insert(name, self.trail.operations.len());
    }

    /// Rewind to checkpoint
    pub fn rewind(&mut self, name: &str) -> Result<(), String> {
        let checkpoint_pos = *self.trail.checkpoints.get(name)
            .ok_or_else(|| format!("Unknown checkpoint: {}", name))?;
        
        // Undo operations back to checkpoint
        while self.trail.operations.len() > checkpoint_pos {
            if let Some(op) = self.trail.operations.pop() {
                self.undo_operation(op);
            }
        }
        
        Ok(())
    }

    /// Rewind last n operations
    pub fn rewind_n(&mut self, n: usize) {
        for _ in 0..n {
            if let Some(op) = self.trail.operations.pop() {
                self.undo_operation(op);
            }
        }
    }

    fn undo_operation(&mut self, op: TrailOp) {
        match op {
            TrailOp::Write { pos, old, .. } => {
                self.head = pos;
                // Write old data without recording to trail
                self.write_raw(&old);
            }
            TrailOp::Seek { old_pos, .. } => {
                self.head = old_pos;
            }
            TrailOp::Mark { label, .. } => {
                self.marks.remove(&label);
            }
            TrailOp::SegmentCreate { .. } => {
                // Segment removal handled by SegmentedTape
            }
            TrailOp::SegmentModify { .. } => {
                // Segment modification handled by SegmentedTape
            }
        }
    }

    fn write_raw(&mut self, data: &[u8]) {
        // Write without recording to trail (for undo operations)
        let mut pos = self.head;
        let mut written = 0;
        
        while written < data.len() {
            let page_idx = pos / 4096;
            let page_offset = (pos % 4096) as usize;
            let to_write = (data.len() - written).min(4096 - page_offset);
            
            let page = self.pages.entry(page_idx).or_insert_with(|| {
                Page {
                    data: Box::new([0; 4096]),
                    cow_refs: 0,
                }
            });
            
            page.data[page_offset..page_offset + to_write]
                .copy_from_slice(&data[written..written + to_write]);
            
            written += to_write;
            pos += to_write as i64;
        }
    }

    /// Get current head position
    pub fn position(&self) -> i64 {
        self.head
    }

    /// Get trail length (for debugging/testing)
    pub fn trail_len(&self) -> usize {
        self.trail.operations.len()
    }
    
    /// Add operation to trail (for segment operations)
    pub fn add_trail_op(&mut self, op: TrailOp) {
        self.trail.operations.push(op);
    }
    
    /// Get a mark position by label
    pub fn get_mark(&self, label: &str) -> Option<i64> {
        self.marks.get(label).copied()
    }
}

impl Trail {
    pub fn new() -> Self {
        Trail {
            operations: Vec::new(),
            checkpoints: HashMap::new(),
        }
    }
}

impl Default for Tape {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_read_write() {
        let mut tape = Tape::new();
        tape.write(&[42, 43, 44]);
        tape.seek(0);
        assert_eq!(tape.read(3), vec![42, 43, 44]);
    }

    #[test]
    fn test_advance_and_write() {
        let mut tape = Tape::new();
        tape.write(&[1]);
        tape.advance(1);
        tape.write(&[2]);
        tape.advance(-1);
        assert_eq!(tape.read(1), vec![1]);
        tape.advance(1);
        assert_eq!(tape.read(1), vec![2]);
    }

    #[test]
    fn test_rewind() {
        let mut tape = Tape::new();
        tape.write(&[10]);
        tape.advance(1);
        tape.write(&[20]);
        
        tape.rewind_n(2);  // Undo write and advance
        assert_eq!(tape.read(1), vec![10]);
        assert_eq!(tape.position(), 0);
    }

    #[test]
    fn test_checkpoint_rewind() {
        let mut tape = Tape::new();
        tape.checkpoint("start".to_string());
        
        tape.write(&[1, 2, 3]);
        tape.advance(3);
        tape.write(&[4, 5, 6]);
        
        tape.rewind("start").unwrap();
        assert_eq!(tape.position(), 0);
        assert_eq!(tape.read(6), vec![0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_marks() {
        let mut tape = Tape::new();
        tape.write(&[1, 2, 3]);
        tape.mark("data_start".to_string());
        tape.advance(10);
        tape.write(&[4, 5, 6]);
        
        tape.seek_mark("data_start").unwrap();
        assert_eq!(tape.read(3), vec![1, 2, 3]);
    }

    #[test]
    fn test_large_write_spanning_pages() {
        let mut tape = Tape::new();
        let data: Vec<u8> = (0..8192).map(|i| (i % 256) as u8).collect();
        
        tape.write(&data);
        tape.seek(0);
        assert_eq!(tape.read(8192), data);
    }
}