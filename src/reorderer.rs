use crate::frame::Frame;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

struct OrderedFrame(Frame);

impl PartialEq for OrderedFrame {
    fn eq(&self, other: &Self) -> bool {
        self.0.index == other.0.index
    }
}

impl Eq for OrderedFrame {}

impl PartialOrd for OrderedFrame {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFrame {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.index.cmp(&other.0.index)
    }
}

pub struct Reorderer {
    heap: BinaryHeap<std::cmp::Reverse<OrderedFrame>>,
    expected_index: usize,
}

impl Reorderer {
    pub fn new() -> Self {
        Reorderer {
            heap: BinaryHeap::new(),
            expected_index: 0,
        }
    }

    pub fn push(&mut self, frame: Frame) {
        self.heap.push(std::cmp::Reverse(OrderedFrame(frame)));
    }

    pub fn pop(&mut self) -> Option<Frame> {
        if let Some(peeked) = self.heap.peek()
            && peeked.0.0.index == self.expected_index
        {
            self.expected_index += 1;
            return self.heap.pop().map(|reversed_item| reversed_item.0.0);
        }
        None
    }
}
