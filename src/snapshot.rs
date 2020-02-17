// Copyright 2019 Fullstop000 <fullstop1005@gmail.com>.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

// Copyright (c) 2011 The LevelDB Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::collections::vec_deque::VecDeque;
use std::sync::Arc;

const MIN_SNAPSHOT: u64 = 0;

/// Abstract handle to particular state of a DB.
/// A `Snapshot` is an immutable object and can therefore be safely
/// accessed from multiple threads without any external synchronization.
#[derive(Clone, Copy)]
pub struct Snapshot {
    // The sequence number pointing to the view of db
    sequence_number: u64,
}

impl Snapshot {
    #[inline]
    pub fn sequence(self) -> u64 {
        self.sequence_number
    }
}

impl From<u64> for Snapshot {
    fn from(src: u64) -> Snapshot {
        Snapshot {
            sequence_number: src,
        }
    }
}

/// Different from the C++ implementation,  a VecDequeue is handled for the SnapshotList because
/// a safe double-linked circular list implementation in Rust is tough and not worth it.
/// Although Rust provides a standard double linked list, use a array based containers are faster.
/// See https://doc.rust-lang.org/std/collections/struct.LinkedList.html
pub struct SnapshotList {
    // The initialized snapshot with `MIN_SNAPSHOT` number.
    first: Arc<Snapshot>,
    // All the newly allocated snapshots.
    // A new snapshot will be `pushed back` into `snapshots`
    // Since a Snapshot will be never altered after allocated, `Arc` is suitable here
    snapshots: VecDeque<Arc<Snapshot>>,
}

impl SnapshotList {
    pub fn new() -> Self {
        let first = Arc::new(MIN_SNAPSHOT.into());
        Self {
            first,
            snapshots: VecDeque::new(),
        }
    }

    /// Returns true if current snapshot list is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    #[inline]
    pub fn oldest(&self) -> Arc<Snapshot> {
        if self.is_empty() {
            self.first.clone()
        } else {
            self.snapshots.front().unwrap().clone()
        }
    }

    #[inline]
    fn newest(&self) -> Arc<Snapshot> {
        if self.is_empty() {
            self.first.clone()
        } else {
            self.snapshots.back().unwrap().clone()
        }
    }

    /// Creates a `Snapshot` and appends it to the end of the list
    pub fn snapshot(&mut self, seq: u64) -> Arc<Snapshot> {
        let last_seq = self.last_seq();
        assert!(seq >= last_seq, "[snapshot] the sequence number must be monotonically increasing : [new: {}], [last: {}]", seq, last_seq);
        if last_seq == seq {
            self.newest()
        } else {
            let s = Arc::new(Snapshot {
                sequence_number: seq,
            });
            self.snapshots.push_back(s.clone());
            s
        }
    }

    /// Remove redundant snapshots
    #[inline]
    pub fn gc(&mut self) {
        self.snapshots.retain(|s| Arc::strong_count(s) > 1)
    }

    #[inline]
    fn last_seq(&self) -> u64 {
        self.snapshots
            .back()
            .map_or(self.first.sequence(), |s| s.sequence_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_new_is_empty() {
        let mut s = SnapshotList::new();
        assert!(s.is_empty());
        assert_eq!(MIN_SNAPSHOT, s.last_seq());
        assert_eq!(MIN_SNAPSHOT, s.snapshot(MIN_SNAPSHOT).sequence());
    }

    #[test]
    #[should_panic]
    pub fn test_panic_oldest_when_empty() {
        let s = SnapshotList::new();
        s.oldest();
    }

    #[test]
    #[should_panic]
    pub fn test_panic_newest_when_empty() {
        let s = SnapshotList::new();
        s.newest();
    }

    #[test]
    pub fn test_append_new_snapshot() {
        let mut s = SnapshotList::new();
        for i in vec![1, 1, 2, 3] {
            let s = s.snapshot(i);
            assert_eq!(s.sequence(), i);
        }
        assert_eq!(1, s.oldest().sequence());
        assert_eq!(3, s.newest().sequence());
    }
}
