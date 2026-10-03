//! The in-memory result of a scan.
//!
//! Nodes are stored in depth-first pre-order with each directory's children
//! sorted by size, largest first. That layout gives three things for free:
//! the subtree of node `i` is the contiguous range `i..=i + desc`, the first
//! child of a directory is `i + 1`, and its next sibling is `i + 1 + desc`.
//! Searching or summarising "everything under this folder" is a slice scan.

use std::cmp::Ordering as CmpOrdering;
use std::collections::HashMap;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub const NODE_DIR: u16 = 0x0001;
pub const NODE_HIDDEN: u16 = 0x0002;
pub const NODE_SYSTEM: u16 = 0x0004;
pub const NODE_REPARSE: u16 = 0x0008;
pub const NODE_LOSSY_NAME: u16 = 0x0010;
pub const NODE_DELETED: u16 = 0x0020;
pub const NODE_VIRTUAL: u16 = 0x0040;
pub const NODE_COMPRESSED: u16 = 0x0080;
pub const NODE_SPARSE: u16 = 0x0100;
pub const NODE_CLOUD: u16 = 0x0200;

pub const NO_PARENT: u32 = u32::MAX;

/// Windows FILE_ATTRIBUTE_* bits mapped onto node flags.
pub fn flags_from_attributes(attrs: u32) -> u16 {
    let mut f = 0;
    if attrs & 0x2 != 0 {
        f |= NODE_HIDDEN;
    }
    if attrs & 0x4 != 0 {
        f |= NODE_SYSTEM;
    }
    if attrs & 0x400 != 0 {
        f |= NODE_REPARSE;
    }
    if attrs & 0x800 != 0 {
        f |= NODE_COMPRESSED;
    }
    if attrs & 0x200 != 0 {
        f |= NODE_SPARSE;
    }
    // OFFLINE, RECALL_ON_OPEN, RECALL_ON_DATA_ACCESS: cloud placeholders.
    if attrs & (0x1000 | 0x4_0000 | 0x40_0000) != 0 {
        f |= NODE_CLOUD;
    }
    f
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Node {
    pub parent: u32,
    name_off: u32,
    name_len: u16,
    pub flags: u16,
    /// Last modification, Unix seconds.
    pub modified: u32,
    /// Number of nodes in the subtree, excluding this one.
    pub desc: u32,
    /// Number of files in the subtree (1 for a file).
    pub files: u32,
    /// Logical size in bytes (sum over the subtree for directories).
    pub size: u64,
    /// Bytes allocated on disk (sum over the subtree for directories).
    pub alloc: u64,
}

impl Node {
    pub fn is_dir(&self) -> bool {
        self.flags & NODE_DIR != 0
    }

    pub fn is_deleted(&self) -> bool {
        self.flags & NODE_DELETED != 0
    }
}

/// Where a tree came from and how long it took.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScanInfo {
    pub root_path: String,
    pub method: ScanMethod,
    pub started_at: i64,
    pub duration_ms: u64,
    pub volume_serial: Option<u64>,
    pub volume_total: Option<u64>,
    pub volume_free: Option<u64>,
    pub cluster_size: Option<u64>,
    /// Entries the scanner could not read (permission denied, etc.).
    pub errors: u64,
    /// True when the tree came from a cached snapshot that may be stale.
    pub from_cache: bool,
    pub separator: char,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScanMethod {
    /// Direct read of the NTFS Master File Table.
    Mft,
    /// MFT snapshot brought up to date from the USN change journal.
    MftIncremental,
    /// Parallel directory traversal.
    Walk,
}

/// Collects nodes in any order, then lays them out in sorted pre-order.
pub struct TreeBuilder {
    parents: Vec<u32>,
    nodes: Vec<Node>,
    names: String,
}

impl TreeBuilder {
    pub fn with_capacity(n: usize) -> Self {
        TreeBuilder { parents: Vec::with_capacity(n), nodes: Vec::with_capacity(n), names: String::new() }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Add a node. `parent` is the index returned by an earlier or later
    /// `add` call, or [`NO_PARENT`] for the root (exactly one).
    #[allow(clippy::too_many_arguments)]
    pub fn add(&mut self, parent: u32, name: &str, is_dir: bool, size: u64, alloc: u64, modified: u32, flags: u16) -> u32 {
        let idx = self.nodes.len() as u32;
        let name_off = self.names.len() as u32;
        let name = truncate_utf8(name, u16::MAX as usize);
        self.names.push_str(name);
        let mut flags = flags & !NODE_DIR;
        if is_dir {
            flags |= NODE_DIR;
        }
        self.nodes.push(Node {
            parent,
            name_off,
            name_len: name.len() as u16,
            flags,
            modified,
            desc: 0,
            files: if is_dir { 0 } else { 1 },
            size: if is_dir { 0 } else { size },
            // A directory's own allocation (its index) counts toward its total.
            alloc,
        });
        self.parents.push(parent);
        idx
    }

    pub fn set_parent(&mut self, idx: u32, parent: u32) {
        self.parents[idx as usize] = parent;
        self.nodes[idx as usize].parent = parent;
    }

    /// Lay the tree out in pre-order and compute subtree totals.
    /// Nodes unreachable from `root` (cycles, dangling parents) are gathered
    /// under a virtual "Orphaned files" directory.
    pub fn finish(mut self, root: u32, info: ScanInfo) -> ScanTree {
        let n = self.nodes.len();
        assert!(n > 0 && (root as usize) < n, "tree needs a root");
        self.parents[root as usize] = NO_PARENT;

        let (mut children, mut starts) = build_csr(&self.parents);
        let mut reached = vec![false; n];
        let mut order = preorder(root, &children, &starts, &mut reached);

        if order.len() < n {
            // Collect everything that is not reachable under one virtual node.
            let orphan = self.nodes.len() as u32;
            let name_off = self.names.len() as u32;
            self.names.push_str("$Orphaned");
            self.nodes.push(Node {
                parent: root,
                name_off,
                name_len: 9,
                flags: NODE_DIR | NODE_VIRTUAL,
                ..Default::default()
            });
            self.parents.push(root);
            for (i, _) in reached.iter().enumerate().filter(|(_, r)| !**r) {
                let p = self.parents[i];
                if p == NO_PARENT || p as usize >= n || !is_dir_node(&self.nodes, p) {
                    self.parents[i] = orphan;
                }
            }
            // Break cycles: any unreached node whose parent chain never ends
            // at the orphan node gets attached directly.
            let total = self.nodes.len();
            let mut state = vec![0u8; total]; // 0 unknown, 1 visiting, 2 ok
            state[root as usize] = 2;
            state[orphan as usize] = 2;
            for i in 0..total {
                let mut path = Vec::new();
                let mut cur = i;
                while state[cur] == 0 {
                    state[cur] = 1;
                    path.push(cur);
                    let p = self.parents[cur];
                    if p == NO_PARENT || p as usize >= total {
                        break;
                    }
                    cur = p as usize;
                }
                if state[cur] == 1 {
                    // Cycle (or dead end): cut at the node we stopped on.
                    self.parents[cur] = orphan;
                }
                for p in path {
                    state[p] = 2;
                }
            }
            (children, starts) = build_csr(&self.parents);
            reached = vec![false; self.nodes.len()];
            order = preorder(root, &children, &starts, &mut reached);
            debug_assert_eq!(order.len(), self.nodes.len());
        }

        // Post-order accumulation: walk the pre-order backwards.
        for &i in order.iter().rev() {
            let node = self.nodes[i as usize];
            let p = self.parents[i as usize];
            if p != NO_PARENT {
                let parent = &mut self.nodes[p as usize];
                parent.size += node.size;
                parent.alloc += node.alloc;
                parent.files += node.files;
                parent.desc += node.desc + 1;
                if node.modified > parent.modified && parent.flags & NODE_VIRTUAL != 0 {
                    parent.modified = node.modified;
                }
            }
        }

        // Sort children by size, then re-run the pre-order on sorted lists.
        for i in 0..self.nodes.len() {
            let (s, e) = (starts[i] as usize, starts[i + 1] as usize);
            let slice = &mut children[s..e];
            let nodes = &self.nodes;
            let names = &self.names;
            slice.sort_unstable_by(|&a, &b| {
                let (na, nb) = (&nodes[a as usize], &nodes[b as usize]);
                nb.size
                    .cmp(&na.size)
                    .then_with(|| node_name(names, na).cmp(node_name(names, nb)))
            });
        }
        let mut reached = vec![false; self.nodes.len()];
        let order = preorder(root, &children, &starts, &mut reached);

        let mut new_index = vec![0u32; self.nodes.len()];
        for (new, &old) in order.iter().enumerate() {
            new_index[old as usize] = new as u32;
        }
        let mut nodes = Vec::with_capacity(order.len());
        let mut names = String::with_capacity(self.names.len());
        for &old in &order {
            let mut node = self.nodes[old as usize];
            let name = node_name(&self.names, &node);
            node.name_off = names.len() as u32;
            names.push_str(name);
            let p = self.parents[old as usize];
            node.parent = if p == NO_PARENT { NO_PARENT } else { new_index[p as usize] };
            nodes.push(node);
        }
        names.shrink_to_fit();
        ScanTree { nodes, names, info, ext_cache: parking_lot::Mutex::new(HashMap::new()) }
    }
}

fn is_dir_node(nodes: &[Node], p: u32) -> bool {
    nodes.get(p as usize).is_some_and(|n| n.is_dir())
}

fn truncate_utf8(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn node_name<'a>(names: &'a str, n: &Node) -> &'a str {
    &names[n.name_off as usize..n.name_off as usize + n.name_len as usize]
}

fn build_csr(parents: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let n = parents.len();
    let mut counts = vec![0u32; n + 1];
    for &p in parents {
        if p != NO_PARENT && (p as usize) < n {
            counts[p as usize] += 1;
        }
    }
    let mut starts = vec![0u32; n + 1];
    for i in 0..n {
        starts[i + 1] = starts[i] + counts[i];
    }
    let mut fill = starts.clone();
    let mut children = vec![0u32; starts[n] as usize];
    for (i, &p) in parents.iter().enumerate() {
        if p != NO_PARENT && (p as usize) < n {
            children[fill[p as usize] as usize] = i as u32;
            fill[p as usize] += 1;
        }
    }
    (children, starts)
}

fn preorder(root: u32, children: &[u32], starts: &[u32], reached: &mut [bool]) -> Vec<u32> {
    let mut order = Vec::with_capacity(reached.len());
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        if reached[i as usize] {
            continue;
        }
        reached[i as usize] = true;
        order.push(i);
        let (s, e) = (starts[i as usize] as usize, starts[i as usize + 1] as usize);
        // Push in reverse so the first (largest) child is visited first.
        for &c in children[s..e].iter().rev() {
            if !reached[c as usize] {
                stack.push(c);
            }
        }
    }
    order
}

/// A completed scan.
pub struct ScanTree {
    nodes: Vec<Node>,
    names: String,
    pub info: ScanInfo,
    ext_cache: parking_lot::Mutex<HashMap<u32, Vec<ExtensionStat>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    pub id: u32,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub alloc: u64,
    pub files: u32,
    pub dirs: u32,
    pub modified: u32,
    pub hidden: bool,
    pub system: bool,
    pub reparse: bool,
    pub cloud: bool,
    pub is_virtual: bool,
    pub children: u32,
    /// Share of the parent's size, 0..1.
    pub fraction: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    #[default]
    Size,
    Alloc,
    Name,
    Modified,
    Files,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TreemapItem {
    pub id: u32,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub ext: Option<String>,
    /// Number of items folded into this one ("and 1,234 smaller items").
    pub folded: u32,
    pub children: Vec<TreemapItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionStat {
    pub ext: String,
    pub count: u64,
    pub size: u64,
    pub alloc: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    /// Case-insensitive substring, or a glob when it contains `*` or `?`.
    pub text: String,
    pub under: Option<u32>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub modified_before: Option<u32>,
    pub modified_after: Option<u32>,
    pub extensions: Vec<String>,
    pub files_only: bool,
    pub dirs_only: bool,
    pub limit: Option<usize>,
    pub sort: SortKey,
    /// Leave out hidden and system items.
    pub exclude_hidden: bool,
}

impl ScanTree {
    pub fn root(&self) -> u32 {
        0
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    pub fn name(&self, id: u32) -> &str {
        node_name(&self.names, &self.nodes[id as usize])
    }

    /// Memory held by the tree, for diagnostics.
    pub fn memory_bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node>() + self.names.capacity()
    }

    pub fn subtree(&self, id: u32) -> std::ops::Range<usize> {
        let n = &self.nodes[id as usize];
        id as usize..id as usize + n.desc as usize + 1
    }

    /// Ids of the live children of a directory, largest first (as scanned).
    pub fn child_ids(&self, id: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let Some(node) = self.nodes.get(id as usize) else { return out };
        let end = id as usize + node.desc as usize + 1;
        let mut c = id as usize + 1;
        while c < end {
            let child = &self.nodes[c];
            if !child.is_deleted() {
                out.push(c as u32);
            }
            c += child.desc as usize + 1;
        }
        out
    }

    pub fn view(&self, id: u32) -> NodeView {
        let n = &self.nodes[id as usize];
        let parent_size = if n.parent == NO_PARENT { n.size } else { self.nodes[n.parent as usize].size };
        let dirs = if n.is_dir() { n.desc.saturating_sub(n.files) } else { 0 };
        NodeView {
            id,
            name: self.name(id).to_string(),
            is_dir: n.is_dir(),
            size: n.size,
            alloc: n.alloc,
            files: n.files,
            dirs,
            modified: n.modified,
            hidden: n.flags & NODE_HIDDEN != 0,
            system: n.flags & NODE_SYSTEM != 0,
            reparse: n.flags & NODE_REPARSE != 0,
            cloud: n.flags & NODE_CLOUD != 0,
            is_virtual: n.flags & NODE_VIRTUAL != 0,
            children: if n.is_dir() { self.child_ids(id).len() as u32 } else { 0 },
            fraction: if parent_size == 0 { 0.0 } else { n.size as f64 / parent_size as f64 },
        }
    }

    fn compare(&self, a: u32, b: u32, key: SortKey) -> CmpOrdering {
        let (na, nb) = (&self.nodes[a as usize], &self.nodes[b as usize]);
        match key {
            SortKey::Size => nb.size.cmp(&na.size),
            SortKey::Alloc => nb.alloc.cmp(&na.alloc),
            SortKey::Modified => nb.modified.cmp(&na.modified),
            SortKey::Files => nb.files.cmp(&na.files),
            SortKey::Name => natural_cmp(self.name(a), self.name(b)),
        }
    }

    /// A page of a directory's children (hidden and system items left out
    /// unless `include_hidden`; they still count in the folder's size).
    pub fn children(&self, id: u32, sort: SortKey, descending: bool, offset: usize, limit: usize, include_hidden: bool) -> (Vec<NodeView>, usize) {
        let mut ids = self.child_ids(id);
        if !include_hidden {
            ids.retain(|&c| self.nodes[c as usize].flags & (NODE_HIDDEN | NODE_SYSTEM) == 0);
        }
        let total = ids.len();
        // Children are stored largest-first already; deletions may have
        // disturbed that, so always sort (cheap for real directories).
        ids.sort_by(|&a, &b| {
            let o = self.compare(a, b, sort);
            // Size-like keys compare largest-first; name ascending.
            let natural_desc = sort != SortKey::Name;
            if descending == natural_desc { o } else { o.reverse() }
        });
        let page = ids.into_iter().skip(offset).take(limit).map(|c| self.view(c)).collect();
        (page, total)
    }

    /// Ancestors from the root down to (and including) `id`.
    pub fn breadcrumbs(&self, id: u32) -> Vec<(u32, String)> {
        let mut chain = Vec::new();
        let mut cur = id;
        while (cur as usize) < self.nodes.len() {
            chain.push((cur, self.name(cur).to_string()));
            let p = self.nodes[cur as usize].parent;
            if p == NO_PARENT {
                break;
            }
            cur = p;
        }
        chain.reverse();
        chain
    }

    /// Full path of a node. The root's name is the scanned root path.
    pub fn path(&self, id: u32) -> String {
        let sep = self.info.separator;
        let crumbs = self.breadcrumbs(id);
        let mut out = String::new();
        for (i, (_, name)) in crumbs.iter().enumerate() {
            if i > 0 && !out.ends_with(sep) {
                out.push(sep);
            }
            out.push_str(name);
        }
        out
    }

    /// Whether every component of the path was decoded exactly (so the
    /// path can be handed back to the OS, e.g. to delete it).
    pub fn path_is_exact(&self, id: u32) -> bool {
        let mut cur = id;
        loop {
            let n = &self.nodes[cur as usize];
            if n.flags & (NODE_LOSSY_NAME | NODE_VIRTUAL) != 0 {
                return false;
            }
            if n.parent == NO_PARENT {
                return true;
            }
            cur = n.parent;
        }
    }

    /// Find a node by path, case-insensitively. The path must start with
    /// the tree's root path.
    pub fn find_path(&self, path: &str) -> Option<u32> {
        let sep = self.info.separator;
        let root_name = self.name(0);
        let norm = |s: &str| s.trim_end_matches(['\\', '/']).to_lowercase();
        let root_norm = norm(root_name);
        let path_norm = path.replace(if sep == '\\' { '/' } else { '\\' }, &sep.to_string());
        let lower = path_norm.to_lowercase();
        let rest = lower.strip_prefix(&root_norm)?;
        if !(rest.is_empty() || rest.starts_with(sep)) {
            return None;
        }
        let mut cur = 0u32;
        for comp in rest.split(sep).filter(|c| !c.is_empty()) {
            cur = self.child_ids(cur).into_iter().find(|&c| self.name(c).to_lowercase() == comp)?;
        }
        Some(cur)
    }

    /// Size-ordered nested items for drawing a treemap. At most `max_items`
    /// rectangles in total; the long tail of each directory is folded.
    ///
    /// The budget is spent largest-first across the whole tree (a priority
    /// queue), not depth-first: a 24 GB `pagefile.sys` at the root always
    /// beats the thousandth file inside the first big folder.
    pub fn treemap(&self, id: u32, max_depth: u32, max_items: usize) -> TreemapItem {
        use std::cmp::Reverse;
        use std::collections::{BinaryHeap, HashSet};
        let root = &self.nodes[id as usize];
        // Smaller than ~0.03% of the view is invisible anyway.
        let min = (root.size / 3000).max(1);
        let mut included: HashSet<u32> = HashSet::new();
        let mut heap: BinaryHeap<(u64, Reverse<u32>, u32)> = BinaryHeap::new();
        let push_children = |heap: &mut BinaryHeap<(u64, Reverse<u32>, u32)>, node: u32, depth: u32| {
            for c in self.child_ids(node) {
                let size = self.nodes[c as usize].size;
                if size >= min {
                    heap.push((size, Reverse(c), depth));
                }
            }
        };
        if root.is_dir() && max_depth > 0 {
            push_children(&mut heap, id, 1);
        }
        let mut budget = max_items.max(1);
        while let Some((_, Reverse(c), depth)) = heap.pop() {
            if budget == 0 {
                break;
            }
            budget -= 1;
            included.insert(c);
            if self.nodes[c as usize].is_dir() && depth < max_depth {
                push_children(&mut heap, c, depth + 1);
            }
        }
        self.treemap_build(id, &included)
    }

    fn treemap_build(&self, id: u32, included: &std::collections::HashSet<u32>) -> TreemapItem {
        let n = &self.nodes[id as usize];
        let name = self.name(id).to_string();
        let mut item = TreemapItem {
            id,
            ext: if n.is_dir() { None } else { extension_of(&name) },
            name,
            size: n.size,
            is_dir: n.is_dir(),
            folded: 0,
            children: Vec::new(),
        };
        if !n.is_dir() {
            return item;
        }
        let mut ids = self.child_ids(id);
        if !ids.iter().any(|c| included.contains(c)) {
            return item;
        }
        ids.sort_by(|&a, &b| self.nodes[b as usize].size.cmp(&self.nodes[a as usize].size));
        let mut folded_size = 0u64;
        let mut folded = 0u32;
        for c in ids {
            if included.contains(&c) {
                item.children.push(self.treemap_build(c, included));
            } else {
                folded_size += self.nodes[c as usize].size;
                folded += 1;
            }
        }
        if folded > 0 && folded_size > 0 {
            item.children.push(TreemapItem {
                id: u32::MAX,
                name: format!("{folded} smaller items"),
                size: folded_size,
                is_dir: false,
                ext: None,
                folded,
                children: Vec::new(),
            });
        }
        item
    }

    /// The `n` largest files under `id`.
    pub fn top_files(&self, id: u32, n: usize) -> Vec<NodeView> {
        let range = self.subtree(id);
        let mut ids: Vec<u32> = range
            .into_par_iter()
            .filter(|&i| {
                let node = &self.nodes[i];
                !node.is_dir() && !node.is_deleted()
            })
            .map(|i| i as u32)
            .collect();
        let n = n.min(ids.len());
        if n == 0 {
            return Vec::new();
        }
        ids.select_nth_unstable_by(n - 1, |&a, &b| self.nodes[b as usize].size.cmp(&self.nodes[a as usize].size));
        ids.truncate(n);
        ids.sort_by(|&a, &b| self.nodes[b as usize].size.cmp(&self.nodes[a as usize].size));
        ids.into_iter().map(|i| self.view(i)).collect()
    }

    /// Size and count per file extension under `id`, largest first.
    pub fn extensions(&self, id: u32) -> Vec<ExtensionStat> {
        if let Some(hit) = self.ext_cache.lock().get(&id) {
            return hit.clone();
        }
        let range = self.subtree(id);
        let map = range
            .into_par_iter()
            .fold(HashMap::<String, ExtensionStat>::new, |mut acc, i| {
                let node = &self.nodes[i];
                if node.is_dir() || node.is_deleted() {
                    return acc;
                }
                let ext = extension_of(self.name(i as u32)).unwrap_or_default();
                let e = acc.entry(ext.clone()).or_insert_with(|| ExtensionStat { ext, count: 0, size: 0, alloc: 0 });
                e.count += 1;
                e.size += node.size;
                e.alloc += node.alloc;
                acc
            })
            .reduce(HashMap::new, |mut a, b| {
                for (k, v) in b {
                    let e = a.entry(k).or_insert_with(|| ExtensionStat { ext: v.ext.clone(), count: 0, size: 0, alloc: 0 });
                    e.count += v.count;
                    e.size += v.size;
                    e.alloc += v.alloc;
                }
                a
            });
        let mut out: Vec<ExtensionStat> = map.into_values().collect();
        out.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.ext.cmp(&b.ext)));
        self.ext_cache.lock().insert(id, out.clone());
        out
    }

    pub fn search(&self, q: &SearchQuery) -> (Vec<NodeView>, usize) {
        let range = match q.under {
            Some(u) if (u as usize) < self.nodes.len() => self.subtree(u),
            _ => self.subtree(0),
        };
        let matcher = NameMatcher::new(&q.text);
        let exts: Vec<String> = q.extensions.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect();
        let mut ids: Vec<u32> = range
            .into_par_iter()
            .filter(|&i| {
                let n = &self.nodes[i];
                if n.is_deleted() || n.flags & NODE_VIRTUAL != 0 || n.parent == NO_PARENT {
                    return false;
                }
                if q.files_only && n.is_dir() || q.dirs_only && !n.is_dir() {
                    return false;
                }
                if q.exclude_hidden && n.flags & (NODE_HIDDEN | NODE_SYSTEM) != 0 {
                    return false;
                }
                if q.min_size.is_some_and(|m| n.size < m) || q.max_size.is_some_and(|m| n.size > m) {
                    return false;
                }
                if q.modified_before.is_some_and(|t| n.modified >= t) || q.modified_after.is_some_and(|t| n.modified <= t) {
                    return false;
                }
                let name = self.name(i as u32);
                if !exts.is_empty() {
                    match extension_of(name) {
                        Some(e) if exts.contains(&e) => {}
                        _ => return false,
                    }
                }
                matcher.matches(name)
            })
            .map(|i| i as u32)
            .collect();
        let total = ids.len();
        let limit = q.limit.unwrap_or(500).min(total);
        if limit > 0 && limit < total {
            ids.select_nth_unstable_by(limit - 1, |&a, &b| self.compare(a, b, q.sort));
            ids.truncate(limit);
        }
        ids.sort_by(|&a, &b| self.compare(a, b, q.sort));
        (ids.into_iter().take(limit).map(|i| self.view(i)).collect(), total)
    }

    /// Iterate live file ids under `id` (used by cleanup and duplicates).
    pub fn files_under(&self, id: u32) -> impl Iterator<Item = u32> + '_ {
        self.subtree(id).filter(move |&i| {
            let n = &self.nodes[i];
            !n.is_dir() && !n.is_deleted()
        }).map(|i| i as u32)
    }

    /// Mark a node and its subtree deleted and subtract it from ancestors.
    pub fn mark_deleted(&mut self, id: u32) {
        let Some(node) = self.nodes.get(id as usize).copied() else { return };
        if node.is_deleted() || node.parent == NO_PARENT {
            return;
        }
        for i in self.subtree(id) {
            self.nodes[i].flags |= NODE_DELETED;
        }
        let mut p = node.parent;
        while p != NO_PARENT {
            let parent = &mut self.nodes[p as usize];
            parent.size = parent.size.saturating_sub(node.size);
            parent.alloc = parent.alloc.saturating_sub(node.alloc);
            parent.files = parent.files.saturating_sub(node.files);
            p = parent.parent;
        }
        self.ext_cache.lock().clear();
    }
}

/// Lower-cased extension without the dot, if the name has one.
pub fn extension_of(name: &str) -> Option<String> {
    let dot = name.rfind('.')?;
    if dot == 0 || dot == name.len() - 1 {
        return None;
    }
    let ext = &name[dot + 1..];
    if ext.len() > 16 || ext.contains(' ') {
        return None;
    }
    Some(ext.to_lowercase())
}

/// Case-insensitive "file2 < file10" ordering.
pub fn natural_cmp(a: &str, b: &str) -> CmpOrdering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return CmpOrdering::Equal,
            (None, _) => return CmpOrdering::Less,
            (_, None) => return CmpOrdering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ai.peek().copied().filter(char::is_ascii_digit) {
                    na.push(c);
                    ai.next();
                }
                let mut nb = String::new();
                while let Some(c) = bi.peek().copied().filter(char::is_ascii_digit) {
                    nb.push(c);
                    bi.next();
                }
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if o != CmpOrdering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                let o = x.to_lowercase().cmp(y.to_lowercase());
                if o != CmpOrdering::Equal {
                    return o;
                }
                ai.next();
                bi.next();
            }
        }
    }
}

/// Matches names against a substring or a `*` / `?` glob, ignoring case.
pub struct NameMatcher {
    pattern: Vec<char>,
    glob: bool,
    needle: String,
}

impl NameMatcher {
    pub fn new(text: &str) -> Self {
        let lower = text.trim().to_lowercase();
        NameMatcher { glob: lower.contains(['*', '?']), pattern: lower.chars().collect(), needle: lower }
    }

    pub fn matches(&self, name: &str) -> bool {
        if self.needle.is_empty() {
            return true;
        }
        if self.glob {
            let name: Vec<char> = name.to_lowercase().chars().collect();
            glob_match(&self.pattern, &name)
        } else if name.is_ascii() && self.needle.is_ascii() {
            ascii_contains_ci(name.as_bytes(), self.needle.as_bytes())
        } else {
            name.to_lowercase().contains(&self.needle)
        }
    }
}

fn ascii_contains_ci(hay: &[u8], needle: &[u8]) -> bool {
    if needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w.iter().zip(needle).all(|(a, b)| a.to_ascii_lowercase() == *b))
}

fn glob_match(p: &[char], s: &[char]) -> bool {
    let (mut pi, mut si) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> ScanInfo {
        ScanInfo {
            root_path: "C:\\".into(),
            method: ScanMethod::Walk,
            started_at: 0,
            duration_ms: 0,
            volume_serial: None,
            volume_total: None,
            volume_free: None,
            cluster_size: None,
            errors: 0,
            from_cache: false,
            separator: '\\',
        }
    }

    /// C:\ { a\ { x.bin 100, y.txt 5 }, b\ { z.bin 50 }, top.iso 70 }
    fn sample() -> ScanTree {
        let mut b = TreeBuilder::with_capacity(8);
        let root = b.add(NO_PARENT, "C:\\", true, 0, 0, 0, 0);
        let a = b.add(root, "a", true, 0, 4096, 10, 0);
        let bdir = b.add(root, "b", true, 0, 0, 20, 0);
        b.add(a, "x.bin", false, 100, 4096, 30, 0);
        b.add(a, "y.TXT", false, 5, 4096, 40, NODE_HIDDEN);
        b.add(bdir, "z.bin", false, 50, 4096, 50, 0);
        b.add(root, "top.iso", false, 70, 4096, 60, 0);
        b.finish(root, info())
    }

    #[test]
    fn totals_and_order() {
        let t = sample();
        let root = t.node(0).unwrap();
        assert_eq!(root.size, 225);
        assert_eq!(root.files, 4);
        assert_eq!(root.desc, 6);
        let kids: Vec<String> = t.child_ids(0).iter().map(|&c| t.name(c).to_string()).collect();
        assert_eq!(kids, vec!["a", "top.iso", "b"]);
        let a = t.find_path("c:\\A").unwrap();
        assert_eq!(t.node(a).unwrap().size, 105);
        assert_eq!(t.node(a).unwrap().alloc, 4096 * 3);
        assert_eq!(t.path(t.find_path("C:\\a\\x.bin").unwrap()), "C:\\a\\x.bin");
        assert_eq!(t.find_path("C:/b/z.bin").map(|i| t.name(i).to_string()), Some("z.bin".into()));
        assert!(t.find_path("C:\\nope").is_none());
    }

    #[test]
    fn queries() {
        let t = sample();
        let top = t.top_files(0, 2);
        assert_eq!(top.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), vec!["x.bin", "top.iso"]);
        let ext = t.extensions(0);
        assert_eq!(ext[0].ext, "bin");
        assert_eq!(ext[0].size, 150);
        assert_eq!(ext[0].count, 2);
        assert!(ext.iter().any(|e| e.ext == "txt"));

        let (res, total) = t.search(&SearchQuery { text: "*.bin".into(), ..Default::default() });
        assert_eq!(total, 2);
        assert_eq!(res[0].name, "x.bin");
        let (res, _) = t.search(&SearchQuery { text: "Y.t".into(), ..Default::default() });
        assert_eq!(res.len(), 1);
        assert!(res[0].hidden);
        let b = t.find_path("C:\\b").unwrap();
        let (res, _) = t.search(&SearchQuery { text: "bin".into(), under: Some(b), ..Default::default() });
        assert_eq!(res.len(), 1);
        let (res, _) = t.search(&SearchQuery { min_size: Some(60), files_only: true, ..Default::default() });
        assert_eq!(res.len(), 2);

        let (page, total) = t.children(0, SortKey::Name, false, 0, 10, true);
        assert_eq!(total, 3);
        let a = t.find_path("C:\\a").unwrap();
        assert_eq!(t.children(a, SortKey::Size, true, 0, 10, true).1, 2);
        assert_eq!(t.children(a, SortKey::Size, true, 0, 10, false).1, 1, "hidden y.TXT left out");
        let (res, _) = t.search(&SearchQuery { text: "y.t".into(), exclude_hidden: true, ..Default::default() });
        assert!(res.is_empty());
        assert_eq!(page.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), vec!["a", "b", "top.iso"]);
        let map = t.treemap(0, 3, 100);
        assert_eq!(map.size, 225);
        assert_eq!(map.children.len(), 3);
    }

    #[test]
    fn treemap_budget_goes_to_the_largest_items_first() {
        // A folder with many mid-sized files, then a big file at the root.
        let mut b = TreeBuilder::with_capacity(64);
        let root = b.add(NO_PARENT, "C:\\", true, 0, 0, 0, 0);
        let users = b.add(root, "Users", true, 0, 0, 0, 0);
        for i in 0..40 {
            b.add(users, &format!("f{i}.bin"), false, 10_000, 0, 0, 0);
        }
        b.add(root, "pagefile.sys", false, 150_000, 0, 0, 0);
        let t = b.finish(root, info());
        let map = t.treemap(0, 4, 10);
        let names: Vec<&str> = map.children.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"pagefile.sys"), "{names:?}");
        let users = map.children.iter().find(|c| c.name == "Users").unwrap();
        assert_eq!(users.children.len(), 9, "8 files plus the folded rest");
        assert_eq!(users.children.last().unwrap().folded, 32);
        assert_eq!(users.children.iter().map(|c| c.size).sum::<u64>(), 400_000);
    }

    #[test]
    fn delete_updates_ancestors() {
        let mut t = sample();
        let x = t.find_path("C:\\a\\x.bin").unwrap();
        t.mark_deleted(x);
        assert_eq!(t.node(0).unwrap().size, 125);
        let a = t.find_path("C:\\a").unwrap();
        assert_eq!(t.node(a).unwrap().files, 1);
        assert!(t.find_path("C:\\a\\x.bin").is_none());
        assert_eq!(t.top_files(0, 1)[0].name, "top.iso");
    }

    #[test]
    fn orphans_and_cycles() {
        let mut b = TreeBuilder::with_capacity(8);
        let root = b.add(NO_PARENT, "/", true, 0, 0, 0, 0);
        let c1 = b.add(root, "c1", true, 0, 0, 0, 0);
        let c2 = b.add(c1, "c2", true, 0, 0, 0, 0);
        b.add(c2, "f", false, 7, 0, 0, 0);
        b.set_parent(c1, c2); // c1 <-> c2 cycle, unreachable from root
        b.add(root, "g", false, 3, 0, 0, 0);
        let t = b.finish(root, ScanInfo { separator: '/', ..info() });
        assert_eq!(t.node(0).unwrap().size, 10);
        assert_eq!(t.len(), 6);
        let orphan = t.child_ids(0).into_iter().find(|&c| t.name(c) == "$Orphaned").unwrap();
        assert_eq!(t.node(orphan).unwrap().size, 7);
        assert!(!t.path_is_exact(orphan));
    }

    #[test]
    fn natural_ordering_and_globs() {
        assert_eq!(natural_cmp("file2", "File10"), CmpOrdering::Less);
        assert_eq!(natural_cmp("a", "A"), CmpOrdering::Equal);
        assert!(NameMatcher::new("*.tmp").matches("X.TMP"));
        assert!(!NameMatcher::new("*.tmp").matches("x.tmpl"));
        assert!(NameMatcher::new("ca?he*").matches("cache2"));
        assert!(NameMatcher::new("ÄBC").matches("xäbcx"));
        assert_eq!(extension_of("archive.tar.GZ"), Some("gz".into()));
        assert_eq!(extension_of(".bashrc"), None);
    }
}
