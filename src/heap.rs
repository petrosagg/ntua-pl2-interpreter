use std::collections::VecDeque;
use std::fmt::{self, Debug};

#[derive(Clone, Copy)]
pub struct Word {
    w: i32,
}

impl Word {
    pub fn from_pointer(ptr: usize) -> Word {
        Word {
            w: (ptr as i32) << 1 | 0,
        }
    }

    pub fn from_int(int: i32) -> Word {
        Word { w: int << 1 | 1 }
    }

    pub fn to_pointer(self) -> usize {
        (self.w >> 1) as usize
    }

    pub fn to_int(self) -> i32 {
        self.w >> 1
    }

    pub fn is_pointer(self) -> bool {
        (self.w & 1) == 0
    }
}

impl Debug for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_pointer() {
            write!(f, "Ptr({})", self.to_pointer())
        } else {
            write!(f, "Int({})", self.to_int())
        }
    }
}

#[derive(Debug)]
pub struct Heap {
    pub heap: Box<[Word]>,
    /// Whether the currently active region is the first half of the heap.
    first_half_active: bool,
    /// The first available free address in the active region.
    free_addr: usize,
}

impl Heap {
    pub fn new(total_words: usize) -> Self {
        let vec = vec![Word { w: 0 }; total_words];
        let memory = vec.into_boxed_slice();

        Heap {
            heap: memory,
            first_half_active: true,
            free_addr: 0,
        }
    }

    pub fn gc<'a>(&mut self, rootset: impl IntoIterator<Item = &'a mut Word>) {
        /// This is our todo list for all the words to to be considered by the garbage collection
        /// algorithm. It can be either a word in the stack of the VM or a slot in the heap. We
        /// can't store a mutable Rust pointer to the heap slot since that prevents us from doing
        /// further modifications to the heap that also require mutable access.
        #[derive(Debug)]
        enum TodoEntry<'a> {
            StackWord(&'a mut Word),
            HeapSlot(usize),
        }
        let mut todo = VecDeque::new();
        todo.extend(rootset.into_iter().map(TodoEntry::StackWord));

        let pivot = self.heap.len() / 2;
        let (from_range, mut next, limit) = match self.first_half_active {
            true => (0..pivot, pivot, self.heap.len()),
            false => (pivot..self.heap.len(), 0, pivot),
        };
        self.first_half_active = !self.first_half_active;

        // Loop invariants:
        //  - `from_range` contains the pointer range of the from-space
        //  - `next` contains the next free slot of the to-space
        //  - `limit` contains one-past the last free slot of the to-space
        while let Some(entry) = todo.pop_front() {
            let word = match &entry {
                TodoEntry::StackWord(word) => &*word,
                TodoEntry::HeapSlot(ptr) => &self.heap[*ptr],
            };
            let ptr = word.to_pointer();
            if from_range.contains(&ptr) {
                // Copy the block to the to space if it's not a forwarding pointer
                if !self.heap[ptr].is_pointer() {
                    let header = self.heap[ptr].to_int();
                    let size = (header >> 8) as usize;
                    assert!(next + size < limit, "GC out of memory");

                    // Copy the block to the to-space and set the forwarding pointer
                    self.heap.copy_within(ptr..ptr + size + 1, next);
                    self.heap[ptr] = Word::from_pointer(next);

                    // Push all from-space pointers we may have copied to our todo
                    for (offset, word) in self.heap[next + 1..next + 1 + size].iter().enumerate() {
                        if word.is_pointer() && from_range.contains(&word.to_pointer()) {
                            todo.push_back(TodoEntry::HeapSlot(next + 1 + offset));
                        }
                    }
                    // Update the next free slot of the to-space
                    next += size + 1;
                }
                // At this point the from-space slot contains the forwarding pointer. We use it to
                // update where the entry points to.
                match entry {
                    TodoEntry::StackWord(word) => *word = self.heap[ptr],
                    TodoEntry::HeapSlot(heap_ptr) => self.heap[heap_ptr] = self.heap[ptr],
                }
            }
        }
        self.free_addr = next;
    }

    // allocate a new block
    pub fn alloc(&mut self, size: usize, tag: u8, words: &[Word]) -> Option<usize> {
        let pivot = self.heap.len() / 2;
        let limit = match self.first_half_active {
            true => pivot,
            false => self.heap.len(),
        };
        if self.free_addr + size >= limit {
            return None;
        }
        let addr = self.free_addr;
        // Store the header for this allocation
        self.heap[addr] = Word::from_int((size << 8 | (tag as usize)).try_into().unwrap());
        self.heap[addr + 1..addr + 1 + size].copy_from_slice(words);
        self.free_addr += size + 1;
        Some(addr)
    }
}
