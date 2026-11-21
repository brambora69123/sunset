//! `PathNode`, `Path` and `BinaryHeap`, from `entity/ai/pathing/`.

use crate::math;

/// `PathNode`: one walkable block position on a path.
#[derive(Clone, Debug)]
pub struct PathNode {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    hash: i32,
    pub heap_index: i32,
    pub distance_from_start: f32,
    pub distance_to_target: f32,
    pub weight: f32,
    /// Previous node on the found path, as a slab index.
    pub prev: Option<usize>,
    pub visited: bool,
}

impl PathNode {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        PathNode {
            x,
            y,
            z,
            hash: PathNode::hash(x, y, z),
            heap_index: -1,
            distance_from_start: 0.0,
            distance_to_target: 0.0,
            weight: 0.0,
            prev: None,
            visited: false,
        }
    }

    /// `PathNode.hash(x, y, z)` — the exact vanilla hash.
    pub fn hash(x: i32, y: i32, z: i32) -> i32 {
        (y & 0xFF)
            | ((x & 32767) << 8)
            | ((z & 32767) << 24)
            | (if x < 0 { i32::MIN } else { 0 })
            | (if z < 0 { 32768 } else { 0 })
    }

    /// `distanceTo(other)`.
    pub fn distance_to(&self, other: &PathNode) -> f32 {
        let dx = (other.x - self.x) as f32;
        let dy = (other.y - self.y) as f32;
        let dz = (other.z - self.z) as f32;
        math::sqrt(dx * dx + dy * dy + dz * dz)
    }

    /// `squaredDistanceTo(other)`.
    pub fn squared_distance_to(&self, other: &PathNode) -> f32 {
        let dx = (other.x - self.x) as f32;
        let dy = (other.y - self.y) as f32;
        let dz = (other.z - self.z) as f32;
        dx * dx + dy * dy + dz * dz
    }

    /// `equals` — hash and coordinates, like vanilla.
    pub fn equals_node(&self, other: &PathNode) -> bool {
        self.hash == other.hash && self.x == other.x && self.y == other.y && self.z == other.z
    }

    /// `isInHeap()`.
    pub fn is_in_heap(&self) -> bool {
        self.heap_index >= 0
    }
}

/// The pathfinder's node slab: nodes keyed by hash, like vanilla's `Int2ObjectHashMap`
/// (colliding hashes share one node).
#[derive(Default)]
pub struct NodeStore {
    pub nodes: Vec<PathNode>,
    by_hash: std::collections::HashMap<i32, usize>,
}

impl NodeStore {
    pub fn new() -> Self {
        NodeStore::default()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.by_hash.clear();
    }

    /// `getOrAddNode(x, y, z)`.
    pub fn get_or_add(&mut self, x: i32, y: i32, z: i32) -> usize {
        let h = PathNode::hash(x, y, z);
        if let Some(&index) = self.by_hash.get(&h) {
            return index;
        }
        let index = self.nodes.len();
        self.nodes.push(PathNode::new(x, y, z));
        self.by_hash.insert(h, index);
        index
    }
}

/// `BinaryHeap` over node slab indices, ordered by node `weight`.
#[derive(Default)]
pub struct BinaryHeap {
    heap: Vec<usize>,
    size: usize,
}

impl BinaryHeap {
    pub fn new() -> Self {
        BinaryHeap {
            heap: vec![usize::MAX; 1024],
            size: 0,
        }
    }

    pub fn clear(&mut self) {
        self.size = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// `insert(node)` — panics when the node is already in the heap ("OW KNOWS!").
    pub fn insert(&mut self, store: &mut NodeStore, node: usize) {
        assert!(store.nodes[node].heap_index < 0, "OW KNOWS!");
        if self.size == self.heap.len() {
            self.heap.resize(self.size << 1, usize::MAX);
        }
        self.heap[self.size] = node;
        store.nodes[node].heap_index = self.size as i32;
        self.size += 1;
        self.up_heap(store, self.size - 1);
    }

    /// `pop()` — removes and returns the lowest-weight node.
    pub fn pop(&mut self, store: &mut NodeStore) -> usize {
        let top = self.heap[0];
        self.size -= 1;
        self.heap[0] = self.heap[self.size];
        if self.size > 0 {
            self.shift_down(store, 0);
        }
        store.nodes[top].heap_index = -1;
        top
    }

    /// `setWeight(node, weight)`.
    pub fn set_weight(&mut self, store: &mut NodeStore, node: usize, weight: f32) {
        let old = store.nodes[node].weight;
        store.nodes[node].weight = weight;
        let index = store.nodes[node].heap_index as usize;
        if weight < old {
            self.up_heap(store, index);
        } else {
            self.shift_down(store, index);
        }
    }

    fn up_heap(&mut self, store: &mut NodeStore, mut index: usize) {
        let node = self.heap[index];
        let weight = store.nodes[node].weight;
        while index > 0 {
            let parent = (index - 1) >> 1;
            let other = self.heap[parent];
            if !(weight < store.nodes[other].weight) {
                break;
            }
            self.heap[index] = other;
            store.nodes[other].heap_index = index as i32;
            index = parent;
        }
        self.heap[index] = node;
        store.nodes[node].heap_index = index as i32;
    }

    fn shift_down(&mut self, store: &mut NodeStore, mut index: usize) {
        let node = self.heap[index];
        let weight = store.nodes[node].weight;
        loop {
            let left = 1 + (index << 1);
            let right = left + 1;
            if left >= self.size {
                break;
            }
            let left_node = self.heap[left];
            let left_weight = store.nodes[left_node].weight;
            let (right_node, right_weight) = if right >= self.size {
                (usize::MAX, f32::INFINITY)
            } else {
                let n = self.heap[right];
                (n, store.nodes[n].weight)
            };

            if left_weight < right_weight {
                if !(left_weight < weight) {
                    break;
                }
                self.heap[index] = left_node;
                store.nodes[left_node].heap_index = index as i32;
                index = left;
            } else {
                if !(right_weight < weight) {
                    break;
                }
                self.heap[index] = right_node;
                store.nodes[right_node].heap_index = index as i32;
                index = right;
            }
        }
        self.heap[index] = node;
        store.nodes[node].heap_index = index as i32;
    }
}

/// `Path`: the found node chain, walked with `advance()`.
#[derive(Clone, Debug, Default)]
pub struct Path {
    nodes: Vec<(i32, i32, i32)>,
    current_index: usize,
    length: usize,
}

impl Path {
    pub fn new(nodes: Vec<(i32, i32, i32)>) -> Self {
        let length = nodes.len();
        Path {
            nodes,
            current_index: 0,
            length,
        }
    }

    /// `advance()`.
    pub fn advance(&mut self) {
        self.current_index += 1;
    }

    /// `isDone()`.
    pub fn is_done(&self) -> bool {
        self.current_index >= self.length
    }

    /// `getTarget()`.
    pub fn target(&self) -> Option<(i32, i32, i32)> {
        if self.length > 0 {
            Some(self.nodes[self.length - 1])
        } else {
            None
        }
    }

    /// `getNode(index)`.
    pub fn node(&self, index: usize) -> (i32, i32, i32) {
        self.nodes[index]
    }

    /// `length()`.
    pub fn length(&self) -> usize {
        self.length
    }

    /// `truncate(length)` — a negative vanilla length means done immediately.
    pub fn truncate(&mut self, length: i32) {
        self.length = length.max(0) as usize;
    }

    /// `getCurrentIndex()`.
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// `setCurrentIndex(index)`.
    pub fn set_current_index(&mut self, index: usize) {
        self.current_index = index;
    }

    /// `getPos(target, index)` — the node's walk target point for a mob of `width`.
    pub fn get_pos(&self, width: f32, index: usize) -> (f64, f64, f64) {
        let (x, y, z) = self.nodes[index];
        let offset = ((width + 1.0) as i32) as f64 * 0.5;
        (x as f64 + offset, y as f64, z as f64 + offset)
    }

    /// `getCurrentPos(target)`.
    pub fn get_current_pos(&self, width: f32) -> (f64, f64, f64) {
        self.get_pos(width, self.current_index)
    }

    /// `equals(other)` — compares the full node arrays, like vanilla.
    pub fn equals_path(&self, other: Option<&Path>) -> bool {
        let Some(other) = other else {
            return false;
        };
        if other.nodes.len() != self.nodes.len() {
            return false;
        }
        self.nodes
            .iter()
            .zip(other.nodes.iter())
            .all(|(a, b)| a == b)
    }

    /// `isOnTarget(pos)`.
    pub fn is_on_target(&self, pos: (f64, f64, f64)) -> bool {
        match self.target() {
            Some(target) => pos.0 as i32 == target.0 && pos.2 as i32 == target.2,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_hash_matches_vanilla() {
        assert_eq!(PathNode::hash(0, 0, 0), 0);
        assert_eq!(PathNode::hash(1, 0, 1), (1 << 8) | (1 << 24));
        assert_eq!(PathNode::hash(-1, 0, 0), i32::MIN | (32767 << 8));
        // (z & 32767) << 24 overflows into the sign bits, like Java int shifts
        assert_eq!(PathNode::hash(0, 0, -1), -16744448);
        assert_eq!(PathNode::hash(0, 1, 0), 1);
        assert_eq!(PathNode::hash(2, 3, 4), 3 | (2 << 8) | (4 << 24));
    }

    #[test]
    fn path_advance_and_truncate() {
        let mut path = Path::new(vec![(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
        assert_eq!(path.length(), 3);
        assert!(!path.is_done());
        path.advance();
        path.advance();
        assert!(!path.is_done());
        path.advance();
        assert!(path.is_done());
        path.set_current_index(0);
        path.truncate(-1);
        assert!(path.is_done());
    }

    #[test]
    fn path_get_pos_centers_by_width() {
        let path = Path::new(vec![(5, 64, 5)]);
        // (int)(0.6 + 1.0) * 0.5 = 1 * 0.5 = 0.5
        assert_eq!(path.get_pos(0.6, 0), (5.5, 64.0, 5.5));
        // (int)(1.4 + 1.0) * 0.5 = 2 * 0.5 = 1.0
        assert_eq!(path.get_pos(1.4, 0), (6.0, 64.0, 6.0));
    }

    #[test]
    fn heap_pops_lowest_weight() {
        let mut store = NodeStore::new();
        let mut heap = BinaryHeap::new();
        let a = store.get_or_add(0, 0, 0);
        let b = store.get_or_add(1, 0, 0);
        let c = store.get_or_add(2, 0, 0);
        store.nodes[a].weight = 5.0;
        store.nodes[b].weight = 1.0;
        store.nodes[c].weight = 3.0;
        heap.insert(&mut store, a);
        heap.insert(&mut store, b);
        heap.insert(&mut store, c);
        assert_eq!(heap.pop(&mut store), b);
        assert_eq!(heap.pop(&mut store), c);
        assert_eq!(heap.pop(&mut store), a);
        assert!(heap.is_empty());
        assert!(!store.nodes[b].is_in_heap());
    }

    #[test]
    fn heap_set_weight_reorders() {
        let mut store = NodeStore::new();
        let mut heap = BinaryHeap::new();
        let a = store.get_or_add(0, 0, 0);
        let b = store.get_or_add(1, 0, 0);
        store.nodes[a].weight = 1.0;
        store.nodes[b].weight = 2.0;
        heap.insert(&mut store, a);
        heap.insert(&mut store, b);
        heap.set_weight(&mut store, a, 9.0);
        assert_eq!(heap.pop(&mut store), b);
        assert_eq!(heap.pop(&mut store), a);
    }

    #[test]
    fn path_equality_compares_full_arrays() {
        let a = Path::new(vec![(0, 0, 0), (1, 0, 0)]);
        let mut b = a.clone();
        assert!(a.equals_path(Some(&b)));
        b.truncate(1);
        // vanilla compares the node array length, not the truncated length
        assert!(a.equals_path(Some(&b)));
        assert!(!a.equals_path(Some(&Path::new(vec![(0, 0, 0)]))));
        assert!(!a.equals_path(None));
    }

    #[test]
    fn node_store_shares_hash_collisions() {
        let mut store = NodeStore::new();
        // (x & 32767) << 8 collides with x + 32768 shifted the same way
        let a = store.get_or_add(0, 0, 0);
        let b = store.get_or_add(32768, 0, 0);
        assert_eq!(a, b, "vanilla's Int2ObjectHashMap keys by hash only");
    }
}
