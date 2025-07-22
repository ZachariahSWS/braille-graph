//! Incremental binning with data passed **by reference**.
//!
//! Cached buckets, O(1) update on scroll.
//! * Strategy::Index   - split by index
//! * Strategy::Time    - split by time
//!
//! Call pattern for smooth scrolling:
//! ```rust
//! // once
//! let mut binner = Binner::new(Strategy::Index);
//!
//! loop {
//!     data.remove(0);              // drop oldest
//!     data.push(new_step);         // push newest
//!     let binned = binner.bin(&data, &cfg);
//!     // render ...
//! }
//! ```

use crate::core::{
    config::Config, constants::BRAILLE_HORIZONTAL_RESOLUTION as HR, data::DataTimeStep,
};

/// Selectable algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strategy {
    Index,
    Time,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::Index
    }
}

/// Cached metadata for one bucket.
#[derive(Clone)]
struct Bucket {
    start: usize, // inclusive
    end: usize,   // exclusive
    min: f64,
    max: f64,
    min_index: usize,
    max_index: usize,
}

/// Stateful binning engine.
pub struct Binner {
    strat: Strategy,
    target: usize, // bins
    buckets: Vec<Bucket>,
    cached: bool,
    last_len: usize,
    last_xrange: Option<(f64, f64)>,
    prev_first_t: Option<f64>, // to detect scroll
    prev_last_t: Option<f64>,
    win: Option<f64>, // size of x_tick
}

impl Binner {
    // --- Helpers ---

    #[inline]
    pub fn new(strat: Strategy) -> Self {
        Self {
            strat,
            target: 0,
            buckets: Vec::new(),
            cached: false,
            last_len: 0,
            last_xrange: None,
            prev_first_t: None,
            prev_last_t: None,
            win: None,
        }
    }

    #[inline]
    fn recompute_extrema(bucket: &mut Bucket, data: &[DataTimeStep]) {
        bucket.min = f64::INFINITY;
        bucket.max = f64::NEG_INFINITY;
        for index in bucket.start..bucket.end {
            let p = &data[index];
            if p.min < bucket.min {
                bucket.min = p.min;
                bucket.min_index = index;
            }
            if p.max > bucket.max {
                bucket.max = p.max;
                bucket.max_index = index;
            }
        }
    }

    fn emit(&self, data: &[DataTimeStep]) -> Vec<DataTimeStep> {
        let mut out = Vec::with_capacity(self.buckets.len());
        for b in &self.buckets {
            let mid = b.start + (b.end - b.start) / 2;
            out.push(DataTimeStep {
                time: data[mid].time,
                min: b.min,
                max: b.max,
            });
        }
        out
    }

    // --- Index Binning ---

    fn bin_index(&mut self, data: &[DataTimeStep]) -> Vec<DataTimeStep> {
        let n = data.len();

        // Early-out: degenerate sizes
        if n == 0 || self.target == 0 || n <= self.target {
            self.cached = false;
            self.buckets.clear();
            self.last_len = n;
            self.prev_first_t = data.first().map(|p| p.time);
            self.prev_last_t = data.last().map(|p| p.time);
            return data.to_vec();
        }

        if !self.cached {
            return self.build_full_index(data);
        }

        // Fast-path detection: constant length & one-step scroll.
        let scrolled = n == self.last_len
            && self.prev_first_t.map_or(false, |prev| prev != data[0].time)
            && self
                .prev_last_t
                .map_or(false, |prev| prev == data[n - 2].time);

        if !scrolled {
            return self.build_full_index(data); // fall back
        }

        // --- Incremental Update ---
        // Shift bucket indices left by one; last bucket extends by one.
        for b in &mut self.buckets {
            b.start -= 1;
            b.end -= 1;
            if b.min_index > 0 {
                b.min_index -= 1;
            }
            if b.max_index > 0 {
                b.max_index -= 1;
            }
        }
        if let Some(last) = self.buckets.last_mut() {
            last.end += 1; // include new element
            let new_index = n - 1;
            let new_p = &data[new_index];
            if new_p.min < last.min {
                last.min = new_p.min;
                last.min_index = new_index;
            }
            if new_p.max > last.max {
                last.max = new_p.max;
                last.max_index = new_index;
            }
        }
        // First bucket: lost element may have held min/max.
        if let Some(first) = self.buckets.first_mut() {
            let lost_min = first.min_index == 0;
            let lost_max = first.max_index == 0;
            if lost_min || lost_max {
                first.min = f64::INFINITY;
                first.max = f64::NEG_INFINITY;
                for index in first.start..first.end {
                    let p = &data[index];
                    if p.min < first.min {
                        first.min = p.min;
                        first.min_index = index;
                    }
                    if p.max > first.max {
                        first.max = p.max;
                        first.max_index = index;
                    }
                }
            }
        }

        self.prev_first_t = Some(data[0].time);
        self.prev_last_t = Some(data[n - 1].time);
        self.last_len = n;
        self.emit(&data)
    }

    // --- Full Rebuild (Index) ---

    fn build_full_index(&mut self, data: &[DataTimeStep]) -> Vec<DataTimeStep> {
        let n = data.len();
        self.buckets.clear();
        self.buckets.reserve(self.target);

        for i in 0..self.target {
            let start = i * n / self.target;
            let end = (i + 1) * n / self.target;
            let slice = &data[start..end];

            let mut low = slice[0].min;
            let mut high = slice[0].max;
            let mut low_index = start;
            let mut high_index = start;
            for (off, p) in slice.iter().enumerate().skip(1) {
                if p.min < low {
                    low = p.min;
                    low_index = start + off;
                }
                if p.max > high {
                    high = p.max;
                    high_index = start + off;
                }
            }

            self.buckets.push(Bucket {
                start,
                end,
                min: low,
                max: high,
                min_index: low_index,
                max_index: high_index,
            });
        }

        self.cached = true;
        self.last_len = n;
        self.prev_first_t = Some(data[0].time);
        self.prev_last_t = Some(data[n - 1].time);
        self.emit(data)
    }

    // --- Uniform Time Binning ---

    fn bin_time(&mut self, data: &[DataTimeStep], cfg: &Config) -> Vec<DataTimeStep> {
        let n = data.len();
        let target = cfg.x_chars * HR;

        // Full rebuild triggers
        let need_full = !self.cached
            || self.target != target
            || self.last_len != n
            || cfg.x_range != self.last_xrange;

        if need_full {
            let t_lo = data.first().unwrap().time;
            let t_hi = data.last().unwrap().time;
            let win = (t_hi - t_lo) / target as f64;

            self.cached = true;
            self.target = target;
            self.last_len = n;
            self.last_xrange = cfg.x_range;
            self.win = Some(win);
            self.prev_first_t = Some(t_lo);
            self.prev_last_t = Some(t_hi);

            return self.build_full_time(data, win);
        }

        // --- Incremental Path ---
        let win = self.win.unwrap(); // cached window width

        // 1. shift every bucket one position to the left and grow it by one elem
        for b in &mut self.buckets {
            b.start -= 1;
            b.end -= 1;
            if b.min_index > 0 {
                b.min_index -= 1;
            }
            if b.max_index > 0 {
                b.max_index -= 1;
            }
        }
        // Append newest element to last bucket
        {
            let new_index = n - 1;
            let p_new = &data[new_index];
            let last = self.buckets.last_mut().unwrap();
            last.end += 1;
            if p_new.min < last.min {
                last.min = p_new.min;
                last.min_index = new_index;
            }
            if p_new.max > last.max {
                last.max = p_new.max;
                last.max_index = new_index;
            }
        }

        // 2. Propagate spills left to right so each bucket covers exactly its
        // time window [t_lo + i * win , t_lo + (i+1) * win)
        let t_lo_new = data.first().unwrap().time;
        for i in 0..self.buckets.len() - 1 {
            loop {
                let window_hi = t_lo_new + (i + 1) as f64 * win;
                let move_condition = {
                    let b = &self.buckets[i];
                    data[b.end - 1].time >= window_hi
                };
                if !move_condition {
                    break;
                }

                // We need simultaneous mutable access to buckets i and i+1.
                let (left_slice, right_slice) = self.buckets.split_at_mut(i + 1);
                let left = &mut left_slice[i];
                let right = &mut right_slice[0];

                // Move the last element of `left` into the front of `right`.
                left.end -= 1;
                right.start -= 1;

                let moved_index = right.start;
                let moved_p = &data[moved_index];

                // Update extrema in `left` if they were lost.
                if left.min_index >= left.end || left.max_index >= left.end {
                    Self::recompute_extrema(left, data);
                }
                // Update extrema in `right` with the inserted element.
                if moved_p.min < right.min {
                    right.min = moved_p.min;
                    right.min_index = moved_index;
                }
                if moved_p.max > right.max {
                    right.max = moved_p.max;
                    right.max_index = moved_index;
                }
            }
        }

        // 3. fix leftmost bucket if it lost extrema due to the global shift
        if let Some(first) = self.buckets.first_mut() {
            if first.min_index < first.start || first.max_index < first.start {
                Self::recompute_extrema(first, data);
            }
        }

        // 4 · update bookkeeping & emit
        self.prev_first_t = Some(t_lo_new);
        self.prev_last_t = Some(data.last().unwrap().time);
        self.last_len = n;
        self.emit(data)
    }

    // --- Full Rebuild (Uniform Time) ---

    fn build_full_time(&mut self, data: &[DataTimeStep], win: f64) -> Vec<DataTimeStep> {
        let target = self.target;
        let t_lo = data.first().unwrap().time;

        self.buckets.clear();
        let mut out: Vec<DataTimeStep> = Vec::with_capacity(target);

        let mut window_lo = t_lo;
        let mut index = 0usize;

        for _ in 0..target {
            let window_hi = window_lo + win;
            let start = index;

            let mut low = f64::INFINITY;
            let mut high = f64::NEG_INFINITY;
            let mut low_index = start;
            let mut high_index = start;

            while index < data.len() && data[index].time < window_hi {
                let p = &data[index];
                if p.min < low {
                    low = p.min;
                    low_index = index;
                }
                if p.max > high {
                    high = p.max;
                    high_index = index;
                }
                index += 1;
            }

            if !low.is_finite() {
                // Empty bucket - duplicate previous or fall back to current index
                if let Some(prev) = out.last() {
                    low = prev.min;
                    high = prev.max;
                } else {
                    let p = &data[index.min(data.len() - 1)];
                    low = p.min;
                    high = p.max;
                }
            }

            self.buckets.push(Bucket {
                start,
                end: index,
                min: low,
                max: high,
                min_index: low_index,
                max_index: high_index,
            });

            out.push(DataTimeStep {
                time: 0.5 * (window_lo + window_hi),
                min: low,
                max: high,
            });

            window_lo = window_hi;
        }
        out
    }

    // --- API ---

    pub fn bin(&mut self, data: &[DataTimeStep], cfg: &Config) -> Vec<DataTimeStep> {
        // Determine current target bin count
        let target = cfg.x_chars * HR;

        // Cache invalidation triggers
        let xrange_changed = cfg.x_range != self.last_xrange;
        if self.strat != Strategy::Index // strategy switch
            || self.target != target     // terminal resize
            || xrange_changed
        // new clip window
        {
            self.cached = false;
            self.buckets.clear();
            self.target = target;
            self.last_xrange = cfg.x_range;
        }

        match self.strat {
            Strategy::Index => self.bin_index(data),
            Strategy::Time => self.bin_time(data, cfg),
        }
    }
}
