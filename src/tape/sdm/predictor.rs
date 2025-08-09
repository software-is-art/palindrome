//! Access pattern prediction for intelligent prefetching
//! 
//! Learns from access patterns to predict future accesses and
//! optimize data placement and prefetching.

use std::collections::{VecDeque, HashMap};

/// Access pattern predictor
#[derive(Debug)]
pub struct AccessPredictor {
    /// Recent access history
    history: VecDeque<AccessRecord>,
    
    /// Markov chain for predicting next access
    markov_chain: MarkovChain,
    
    /// Sequential access detector
    sequential_detector: SequentialDetector,
    
    /// Temporal pattern detector (for time-travel)
    temporal_detector: TemporalDetector,
    
    /// Configuration
    config: PredictorConfig,
}

/// Record of a single access
#[derive(Debug, Clone)]
pub struct AccessRecord {
    /// Position accessed
    pub position: i64,
    
    /// Length of access
    pub length: usize,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Was this a write?
    pub is_write: bool,
}

/// Markov chain for access prediction
#[derive(Debug)]
pub struct MarkovChain {
    /// Transition matrix: current_page -> (next_page, count)
    transitions: HashMap<i64, HashMap<i64, u32>>,
    
    /// Total transitions from each page
    totals: HashMap<i64, u32>,
}

/// Sequential access pattern detector
#[derive(Debug)]
pub struct SequentialDetector {
    /// Recent sequential runs
    runs: VecDeque<SequentialRun>,
    
    /// Current run being tracked
    current_run: Option<SequentialRun>,
    
    /// Last accessed page
    last_page: Option<i64>,
}

/// A run of sequential accesses
#[derive(Debug, Clone)]
pub struct SequentialRun {
    /// Starting position
    pub start: i64,
    
    /// Current position
    pub current: i64,
    
    /// Stride (usually 1 for forward, -1 for backward)
    pub stride: i64,
    
    /// Number of accesses in this run
    pub length: u32,
}

/// Temporal pattern detector for time-travel
#[derive(Debug)]
pub struct TemporalDetector {
    /// Checkpoint access patterns
    checkpoint_patterns: HashMap<String, Vec<i64>>,
    
    /// Rewind patterns
    rewind_history: VecDeque<RewindEvent>,
}

/// A rewind event
#[derive(Debug, Clone)]
pub struct RewindEvent {
    /// Pages accessed before rewind
    pub before: Vec<i64>,
    
    /// Pages accessed after rewind
    pub after: Vec<i64>,
    
    /// Timestamp of rewind
    pub timestamp: u64,
}

/// Predictor configuration
#[derive(Debug, Clone)]
pub struct PredictorConfig {
    /// Maximum history size
    pub max_history: usize,
    
    /// Minimum confidence for predictions
    pub min_confidence: f32,
    
    /// Sequential threshold (how many in a row = sequential)
    pub sequential_threshold: u32,
    
    /// Markov chain order (1 = first-order)
    pub markov_order: u32,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        PredictorConfig {
            max_history: 1000,
            min_confidence: 0.3,
            sequential_threshold: 3,
            markov_order: 1,
        }
    }
}

impl AccessPredictor {
    /// Create a new access predictor
    pub fn new() -> Self {
        Self::with_config(PredictorConfig::default())
    }
    
    /// Create with custom configuration
    pub fn with_config(config: PredictorConfig) -> Self {
        AccessPredictor {
            history: VecDeque::with_capacity(config.max_history),
            markov_chain: MarkovChain::new(),
            sequential_detector: SequentialDetector::new(),
            temporal_detector: TemporalDetector::new(),
            config,
        }
    }
    
    /// Record an access
    pub fn record_access(&mut self, position: i64, length: usize, is_write: bool) {
        let record = AccessRecord {
            position,
            length,
            timestamp: current_timestamp(),
            is_write,
        };
        
        // Update history
        self.history.push_back(record.clone());
        if self.history.len() > self.config.max_history {
            self.history.pop_front();
        }
        
        // Update detectors
        let page = position / 4096; // Assuming 4KB pages
        self.markov_chain.record_transition(self.last_page(), page);
        self.sequential_detector.record_access(page);
    }
    
    /// Suggest pages to prefetch
    pub fn suggest_prefetch(&self, current_page: i64) -> Option<Vec<i64>> {
        let mut suggestions = Vec::new();
        
        // Check for sequential pattern
        if let Some(pages) = self.sequential_detector.predict_next(current_page, 5) {
            suggestions.extend(pages);
        }
        
        // Use Markov chain for additional predictions
        let markov_predictions = self.markov_chain.predict_next(current_page, self.config.min_confidence);
        for (page, confidence) in markov_predictions {
            if confidence > self.config.min_confidence && !suggestions.contains(&page) {
                suggestions.push(page);
            }
        }
        
        if suggestions.is_empty() {
            None
        } else {
            Some(suggestions)
        }
    }
    
    /// Check if access pattern is sequential
    pub fn is_sequential(&self) -> bool {
        self.sequential_detector.current_run.as_ref()
            .map(|run| run.length >= self.config.sequential_threshold)
            .unwrap_or(false)
    }
    
    /// Record a checkpoint event
    pub fn record_checkpoint(&mut self, name: String) {
        // Record pages accessed recently (potential rewind targets)
        let recent_pages: Vec<i64> = self.history.iter()
            .rev()
            .take(50)
            .map(|r| r.position / 4096)
            .collect();
        
        self.temporal_detector.checkpoint_patterns.insert(name, recent_pages);
    }
    
    /// Record a rewind event
    pub fn record_rewind(&mut self, pages_before: Vec<i64>, pages_after: Vec<i64>) {
        let event = RewindEvent {
            before: pages_before,
            after: pages_after,
            timestamp: current_timestamp(),
        };
        
        self.temporal_detector.rewind_history.push_back(event);
        if self.temporal_detector.rewind_history.len() > 100 {
            self.temporal_detector.rewind_history.pop_front();
        }
    }
    
    /// Predict pages likely to be accessed after a rewind
    pub fn predict_rewind_targets(&self) -> Vec<i64> {
        let mut page_scores: HashMap<i64, u32> = HashMap::new();
        
        // Analyze rewind history
        for event in &self.temporal_detector.rewind_history {
            for page in &event.after {
                *page_scores.entry(*page).or_insert(0) += 1;
            }
        }
        
        // Sort by score and return top predictions
        let mut predictions: Vec<(i64, u32)> = page_scores.into_iter().collect();
        predictions.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        
        predictions.into_iter()
            .take(10)
            .map(|(page, _)| page)
            .collect()
    }
    
    /// Get the last accessed page
    fn last_page(&self) -> i64 {
        self.history.back()
            .map(|r| r.position / 4096)
            .unwrap_or(0)
    }
}

impl MarkovChain {
    fn new() -> Self {
        MarkovChain {
            transitions: HashMap::new(),
            totals: HashMap::new(),
        }
    }
    
    fn record_transition(&mut self, from: i64, to: i64) {
        let transitions = self.transitions.entry(from).or_insert_with(HashMap::new);
        *transitions.entry(to).or_insert(0) += 1;
        *self.totals.entry(from).or_insert(0) += 1;
    }
    
    fn predict_next(&self, current: i64, min_confidence: f32) -> Vec<(i64, f32)> {
        if let Some(transitions) = self.transitions.get(&current) {
            let total = self.totals.get(&current).unwrap_or(&1);
            
            let mut predictions: Vec<(i64, f32)> = transitions.iter()
                .map(|(next, count)| (*next, *count as f32 / *total as f32))
                .filter(|(_, conf)| *conf >= min_confidence)
                .collect();
            
            predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            predictions
        } else {
            Vec::new()
        }
    }
}

impl SequentialDetector {
    fn new() -> Self {
        SequentialDetector {
            runs: VecDeque::with_capacity(10),
            current_run: None,
            last_page: None,
        }
    }
    
    fn record_access(&mut self, page: i64) {
        if let Some(ref mut run) = self.current_run {
            if run.stride == 0 && run.length == 1 {
                // First time, establish stride
                let stride = page - run.current;
                if stride.abs() <= 10 && stride != 0 {
                    run.stride = stride;
                    run.current = page;
                    run.length = 2;
                } else {
                    // Not sequential, restart
                    self.current_run = Some(SequentialRun {
                        start: page,
                        current: page,
                        stride: 0,
                        length: 1,
                    });
                }
            } else {
                let expected_next = run.current + run.stride;
                
                if page == expected_next {
                    // Continue the run
                    run.current = page;
                    run.length += 1;
                } else if run.length >= 3 {
                // End the run and save it
                self.runs.push_back(run.clone());
                if self.runs.len() > 10 {
                    self.runs.pop_front();
                }
                
                // Start a new run
                self.current_run = Some(SequentialRun {
                    start: page,
                    current: page,
                    stride: 0,
                    length: 1,
                });
            } else {
                // Too short, restart
                self.current_run = Some(SequentialRun {
                    start: page,
                    current: page,
                    stride: 0,
                    length: 1,
                });
                }
            }
        } else if let Some(last) = self.last_page {
            // Check if this could be the start of a sequential run
            let stride = page - last;
            if stride.abs() <= 10 {
                self.current_run = Some(SequentialRun {
                    start: last,
                    current: page,
                    stride,
                    length: 2,
                });
            }
        } else {
            // First access
            self.current_run = Some(SequentialRun {
                start: page,
                current: page,
                stride: 0,
                length: 1,
            });
        }
        
        // Update last page
        self.last_page = Some(page);
    }
    
    fn predict_next(&self, current: i64, count: usize) -> Option<Vec<i64>> {
        if let Some(ref run) = self.current_run {
            if run.current == current && run.length >= 3 {
                // Predict continuation of current run
                let mut predictions = Vec::with_capacity(count);
                let mut next = current;
                
                for _ in 0..count {
                    next += run.stride;
                    predictions.push(next);
                }
                
                return Some(predictions);
            }
        }
        
        None
    }
    
}

impl TemporalDetector {
    fn new() -> Self {
        TemporalDetector {
            checkpoint_patterns: HashMap::new(),
            rewind_history: VecDeque::with_capacity(100),
        }
    }
}

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
    fn test_sequential_detection() {
        let mut predictor = AccessPredictor::new();
        
        // Record sequential accesses
        for i in 0..5 {
            predictor.record_access(i * 4096, 4096, false);
        }
        
        assert!(predictor.is_sequential());
        
        // Predict next pages
        let suggestions = predictor.suggest_prefetch(4);
        assert!(suggestions.is_some());
        let pages = suggestions.unwrap();
        assert!(pages.contains(&5));
    }
    
    #[test]
    fn test_markov_prediction() {
        let mut chain = MarkovChain::new();
        
        // Record pattern: 1 -> 2 -> 3 -> 1
        chain.record_transition(1, 2);
        chain.record_transition(2, 3);
        chain.record_transition(3, 1);
        chain.record_transition(1, 2);
        
        let predictions = chain.predict_next(1, 0.5);
        assert!(!predictions.is_empty());
        assert_eq!(predictions[0].0, 2); // Should predict 2 after 1
    }
}