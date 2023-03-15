use std::{hash::Hash, collections::{BinaryHeap, HashSet}};


pub struct HashHeap<T> where T: Ord + Hash + Copy + Clone {
    heap: BinaryHeap<T>,
    tracker: HashSet<T>,
    capacity: usize
}

impl<T> HashHeap<T> where T: Ord + Hash + Copy + Clone {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { heap: BinaryHeap::with_capacity(capacity+1), tracker: HashSet::with_capacity(capacity+1), capacity }
    }

    pub fn insert(&mut self, item: T) -> bool {
        if !self.tracker.contains(&item) {
            self.tracker.insert(item);
            self.heap.push(item);
            if self.heap.len() > self.capacity {
                let popped = match self.heap.pop(){
                    Some(item) => item,
                    None => return false
                };
                self.tracker.remove(&popped);
            }
            return true
        }
        false
    }

    pub fn into_sorted_vec(&self) -> Vec<T> {
        self.heap.clone().into_sorted_vec()
    }
}