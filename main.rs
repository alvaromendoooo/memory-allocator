use std::cmp::min;
use std::io::{self, BufRead};
use std::str::FromStr;
use std::collections::{HashMap, BTreeMap};

#[derive(Debug)]
pub enum AllocatorEngine { // Lets me keep the logic of incompatible allocator algorithms
    Knuth(FitStratController),
    Buddy(BudyAllocator),
    Slab(HashMap<String, SlabAllocator>),
    ThreadLocalCache(ThreadCacheManager),
}

#[derive(Debug, PartialEq)]
pub enum AllowedInstructions {
    INIT,
    ALLOC,
    USED,
    RESET,
    FREE,
    BLOCKS,
    FREELIST,
    COUNT,
    PREV,
    STATS,
    ORDER,
    BUDDY,
    CACHE_CREATE,
    REPORT,
}

#[derive(Debug)]
pub enum AvailableAllocators {
    BUMP,
    SLAB,
    TCMALLOC,
    ARENA,
    DEFAULT
}

#[derive(Debug, PartialEq)]
pub enum FitStrategies {
    FIRST,
    BEST,
    WORST
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Used,
    Free
}

// Concret struct to handle parsing str->enum errors
#[derive(Debug)]
pub struct ParseInstructionError;

#[derive(Debug)]
pub struct ParseAllocatorSelectedError;

// Definition of memory block components
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub data_address: i32,
    pub size: i32,
    pub status: Status,
    pub class: Option<i32>,
    pub req_size: i32,
}

// Definition of an allocator record to keep track of changes during runtime - SlabAllocator
#[derive(Debug, Clone)]
pub struct AllocRecord {
    pub name: String,
    pub slab: usize,
    pub slot: usize
}

// Single slab containing fixed size slots
#[derive(Debug)]
pub struct Slab {
    //pub slot_size: usize, - Not needed now
    pub total_slots: usize,
    pub free_slots: Vec<bool>,
}

// Defiition of Classes for allocating memory
#[derive(Debug)]
pub struct Class {
    pub sizes: Vec<i32>,
    pub alloc_count: Vec<i32>,
    pub free_count: Vec<i32>,
}

// Definition of allocator instance, contolling memory block registry
#[derive(Debug)]
pub struct Allocator {
    pub blocks: Vec<MemoryBlock>,
    pub class: Class,
    pub internal_frag: i32,
}

// Definition of memory strategy allocator manager for Knuth algorithm - Controls ALLOC depending of the strat
#[derive(Debug)]
pub struct FitStratController {
    pub allocator: Allocator,
    pub strat: FitStrategies
}

// Definition of memory buddy allocator manager for Buddy algorithm
#[derive(Debug)]
pub struct BudyAllocator {
    pub max_order: usize,
    pub free_lists: Vec<Vec<i32>>, // Contains free lists inside other free lists
    pub allocated: HashMap<i32, usize> // Maps addr & order for allocated memory in blocks
}

// Definition of memory slab allocator manager for Slab algorithm
#[derive(Debug)]
pub struct SlabAllocator {
    pub name: String,
    pub obj_size: usize,
    pub slab_obj_count: usize,
    pub slabs: Vec<Slab>,
    pub records: HashMap<String, AllocRecord>,
    pub allocator_count: i32,
}

// Definition of Thread cache manager
#[derive(Debug, Default)]
pub struct ThreadCacheManager {
    // Stores how many blocks are availabe per class in central pool
    pub central: HashMap<i32, usize>,

    // Per thread, per class -> blocks count: tid -> (class -> block_count)
    pub threads: HashMap<i32, BTreeMap<i32, usize>>,
}

#[derive(Debug)]
pub struct WorkloadRule {
    pub patterns: Vec<String>,
    pub allocator: String,
}

// Definition of available allocator workloads per pattern description
#[derive(Debug)]
pub struct AllocatorsWorkload {
    pub rules: Vec<WorkloadRule> 
}

// Mapper that converts input str into enum for match iteration control
impl FromStr for AllowedInstructions {     

    type Err = ParseInstructionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "INIT"         => Ok(AllowedInstructions::INIT),
            "ALLOC"        => Ok(AllowedInstructions::ALLOC),
            "USED"         => Ok(AllowedInstructions::USED),
            "RESET"        => Ok(AllowedInstructions::RESET),
            "FREE"         => Ok(AllowedInstructions::FREE),
            "BLOCKS"       => Ok(AllowedInstructions::BLOCKS),
            "FREELIST"     => Ok(AllowedInstructions::FREELIST),
            "PREV"         => Ok(AllowedInstructions::PREV),
            "COUNT"        => Ok(AllowedInstructions::COUNT),
            "STATS"        => Ok(AllowedInstructions::STATS),
            "ORDER"        => Ok(AllowedInstructions::ORDER),
            "BUDDY"        => Ok(AllowedInstructions::BUDDY),
            "CACHE_CREATE" => Ok(AllowedInstructions::CACHE_CREATE),
            "REPORT"       => Ok(AllowedInstructions::REPORT),
            _              => Err(ParseInstructionError),
        }
    }
}

// Mapper converting allocato workload selection string into enum for future match iteration control
impl FromStr for AvailableAllocators {

    type Err = ParseAllocatorSelectedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "BUMP"     => Ok(AvailableAllocators::BUMP),
            "SLAB"     => Ok(AvailableAllocators::SLAB),
            "TCMALLOC" => Ok(AvailableAllocators::TCMALLOC),
            "ARENA"    => Ok(AvailableAllocators::ARENA),
            "DEFAULT"  => Ok(AvailableAllocators::DEFAULT),
            _          => Err(ParseAllocatorSelectedError),
        }
    }
}

// Mapper that converts input str into enum for match strategy control
impl FromStr for FitStrategies {
    type Err = ParseInstructionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "FIRST" => Ok(FitStrategies::FIRST),
            "BEST"  => Ok(FitStrategies::BEST),
            "WORST" => Ok(FitStrategies::WORST),
            _       => Err(ParseInstructionError),
        }
    }
}

// Functionalities for available allocators workloads
impl AllocatorsWorkload {
    pub fn init() -> Self {

        let rules = vec![
            WorkloadRule {
                patterns: vec!["never free".to_string(), "one-shot".to_string(), "bump".to_string()],
                allocator: "bump".to_string(),
            },
            WorkloadRule {
                patterns: vec!["kernel".to_string(), "dentry".to_string(), "inode".to_string(), "fixed-type".to_string()],
                allocator: "slab".to_string(),
            },
            WorkloadRule {
                patterns: vec!["multi-thread*".to_string(), "thread-per-request".to_string(), "web server".to_string(), "tcmalloc".to_string()],
                allocator: "tcmalloc".to_string(),
            },
            WorkloadRule {
                patterns: vec!["per-frame".to_string(), "request handler".to_string(), "parser".to_string()],
                allocator: "arena".to_string()
            }
        ];
        Self { rules }
    }

    pub fn match_pattern(&self, input: &str) -> String {
        let input_lower = input.to_lowercase();

        for rule in &self.rules {
            for pattern in &rule.patterns {
                if input_lower.contains(pattern) {
                    return rule.allocator.to_string();
                }
            }
        }
        "default".to_string()
    }
}

// Functionalities for a slab
impl Slab {
    // New instance
    pub fn new(total_slots: usize) -> Self {
        Self {
            total_slots,
            free_slots: vec![true; total_slots] // All slots are free in first place
        }
    }

    // Finds first available slot in a slab
    pub fn alloc_slot(&mut self) -> Option<usize> {
        if let Some(slot_idx) = self.free_slots.iter().position(|&s| s) {
            self.free_slots[slot_idx] = false; // Now it will be filled
            Some(slot_idx)
        } else {
            None // Slab is full
        }
    }

    // Frees a specific slot from a slab
    pub fn free_slot(&mut self, slot_idx: usize) -> bool {
        if slot_idx < self.total_slots && !self.free_slots[slot_idx] {
            self.free_slots[slot_idx] = true;
            true
        } else {
            false
        }
    }
}

// Slab allocator functionalities
impl SlabAllocator {
    // New instance
    pub fn new(name: String, obj_size: usize, slab_obj_count: usize) -> Self {
        Self {
            name,
            obj_size,
            slab_obj_count,
            slabs: Vec::new(),
            records: HashMap::new(),
            allocator_count: 0
        }
    }
    
    // Alloc a contiguous run of memory pre-divided into fixed size objects of slots
    pub fn alloc(&mut self, name: String) -> Result<String, &'static str> {
       // Find a slot in a existing slab
        for (slab_idx, slab) in self.slabs.iter_mut().enumerate() {
            if let Some(slot_idx) = slab.alloc_slot() {
                let record = AllocRecord {
                    name: name.clone(),
                    slab: slab_idx,
                    slot: slot_idx
                };

                let record_str = format!("{}:{}", record.slab, record.slot);
                self.records.insert(format!("{}@{}", name, record_str), record);
                self.allocator_count += 1;
                return Ok(record_str)
            }
        }

        // If all slots are full, we need to create a new slab
        let mut new_slab = Slab::new(self.slab_obj_count);
        let slot_idx = new_slab.alloc_slot().unwrap();
        let slab_idx = self.slabs.len();
        self.slabs.push(new_slab);

        let record = AllocRecord {
            name: name.clone(),
            slab: slab_idx,
            slot: slot_idx
        };

        let record_str = format!("{}:{}", record.slab, record.slot);
        self.records.insert(format!("{}@{}", name, record_str), record);
        self.allocator_count += 1;
        Ok(record_str)

    }

    pub fn free(&mut self, name: String, slab: usize, slot: usize) -> String {

        if let Some(slab_related) = self.slabs.get_mut(slab) {
            if slab_related.free_slot(slot) {
                let deleted_key = format!("{}@{}:{}", name, slab, slot);
                if self.records.remove(&deleted_key).is_some() {
                    self.allocator_count -= 1;
                    "OK".to_string()
                } else {
                    "BAD".to_string()
                }
            } else {
                "BAD".to_string()
            }
            
        } else {
            "BAD".to_string()
        }
    }

    // Shows info about cache object created
    pub fn stats(&self, name: String) -> String {
        // Count how many records belong to this name
        let objs_alloc = self.records.values()
            .filter(|r| r.name == name)
            .count();

        let mut empty = 0;
        let mut full = 0;
        let mut partial = 0;

        for s in &self.slabs {
            let has_free = s.free_slots.iter().any(|&slot| slot);
            let has_used = s.free_slots.iter().any(|&slot| !slot);

            if has_free && !has_used {
                empty += 1;
            } else if has_used && !has_free {
                full += 1;
            } else if has_free && has_used {
                partial += 1;
            }
        }

        format!(
            "objs_alloc={} slabs={} empty={} partial={} full={}",
            objs_alloc,
            self.slabs.len(),
            empty,
            partial,
            full
        )
    }
}

// Buddy allocator functionalities
impl BudyAllocator {
    // New instance
    pub fn new(max_order: usize) -> Self {
        let mut free_lists = vec![Vec::new(); max_order + 1];
        free_lists[max_order].push(0); // Initial full heap block at order max_order
        Self {
            max_order,
            free_lists,
            allocated: HashMap::new(),
        }
    }

    // Alloc memory inside blocks
    pub fn alloc(&mut self, size: usize) -> Result<i32, &'static str> {
        let req_order =  (size as f64).log2().ceil() as usize;
        if req_order > self.max_order {
            return Err("TOO_LARGE")
        }
        
        // Find the smallest order >= req_order with an empty list
        let mut target_order = None;
        for order in req_order..=self.max_order {
            if !self.free_lists[order].is_empty() {
                target_order = Some(order);
                break;
            }
        }

        let mut current_order = match target_order {
            Some(o) => o,
            None           => return Err("OOM")
        };

        // Pops block from target order
        let addr = self.free_lists[current_order].pop().unwrap();

        // Split lists repetedly until requested size fits in the best block possible
        while current_order > req_order {
            current_order -= 1;
            let buddy_addr = addr + (1 << current_order);
            self.free_lists[current_order].push(buddy_addr);
        }

        self.allocated.insert(addr, req_order);
        Ok(addr)
    }

    // Free memory considering coelescing buddy's block
    pub fn free(&mut self, addr: i32) -> Result<&'static str, &'static str> {
        let mut order = match self.allocated.remove(&addr) {
            Some(o) => o,
            None           => return Err("BAD"),
        };

        let mut current_addr = addr;

        while order < self.max_order {
            let buddy_addr = current_addr ^ (1 << order);
            if let Some(idx) = self.free_lists[order].iter().position(|&a| a == buddy_addr) {
                self.free_lists[order].remove(idx);
                current_addr = min(current_addr, buddy_addr);
                order += 1;
            } else {
                break;
            }
        }

        self.free_lists[order].push(current_addr);
        Ok("OK")
    }

    // Free lists behavior implementation
    pub fn free_lists(&self, order: usize) -> String {
        if order > self.max_order {
            return String::new();
        }

        let mut list = self.free_lists[order].clone();
        list.sort();
        list.iter()
            .map(|a| a.to_string())
            .collect::<Vec<String>>()
            .join(",")
    }

    pub fn order(&self, size: usize) -> String {
        let k = (size as f64).log2().ceil() as usize;
        k.to_string()
    }

    pub fn buddy(&self, addr: i32, order: usize) -> String {
        (addr ^ (1 << order)).to_string()
    }

    
}

// ALLOC Strategy controller functionalities
impl FitStratController {
    // New instance
    pub fn new(allocator: Allocator, strategy: FitStrategies) -> Self {
        Self {
            allocator,
            strat: strategy
        }
    }

    // allco handler that use strategy impl depeding of requirements
    pub fn alloc(&mut self, request_size: i32) -> Result<i32, &'static str> {
        match self.strat {
            FitStrategies::FIRST => self.alloc_first_fit(request_size),
            FitStrategies::BEST  => self.alloc_best_fit(request_size),
            FitStrategies::WORST => self.alloc_worst_fit(request_size)
        }
    }

    // First fit strategy
    pub fn alloc_first_fit(&mut self, size: i32) -> Result<i32, &'static str> {
        let block_idx = self.allocator.blocks.iter().position(|b| {
            b.status == Status::Free && b.size >= size
        });

        self.allocator.alloc_by_idx(size, block_idx)
    }

    // Best fit strategy
    pub fn alloc_best_fit(&mut self, size: i32) -> Result<i32, &'static str> {
        let block_idx = self.allocator.blocks.iter()
            .enumerate() // return tuple (idx, block_reference)
            .filter(|(_, b)| b.status == Status::Free && b.size >= size)
            .min_by_key(|(_, b)| b.size)
            .map(|(idx, _)| idx); // converts Option<(usize, &MemoryBlock)> into
            // Option<usize> with only the block index

        self.allocator.alloc_by_idx(size, block_idx)
    }

    // Worst fit strategy
    pub fn alloc_worst_fit(&mut self, size: i32) -> Result<i32, &'static str> {
        let block_idx = self.allocator.blocks.iter()
            .enumerate()
            .filter(|(_, b)| b.status == Status::Free && b.size >= size)
            .max_by_key(|(_, b)| b.size)
            .map(|(idx, _)| idx);

        self.allocator.alloc_by_idx(size, block_idx)
    }
}

// Class functionalities
impl Class {
    // New instace
    pub fn new() -> Self {
        let sizes =  vec![16, 32, 64, 128, 256, 512, 1024];
        let len = sizes.len();
        Self {
            sizes,
            alloc_count: vec![0; len],
            free_count:  vec![0; len],
        }
    }
}

// Allocator functionalities
impl Allocator {
    // New instace
    pub fn new(class_instance: Class) -> Self {
        Self {
            blocks: Vec::new(),
            class: class_instance,
            internal_frag: 0,
        }
    }

    // Create a new instance of Allocator with free data
    pub fn init(heap_size: i32, class_instance: Class) -> Self {
        let initial_free_block = MemoryBlock {
            //data_address: hdr, - HEADER applied - received from param
            //size: heap_size - hdr - ftr, - HEADER + FOOTER applied - received from param
            data_address: 0,
            size: heap_size,
            status: Status::Free,
            class: None,
            req_size: 0,
        };

        Self {
            blocks: vec![initial_free_block],
            class: class_instance,
            internal_frag: 0,
        }
    }

    // Method to use when printing block
    pub fn print_blocks(&self) -> Vec<String>{
        let mut blocks_stringify: Vec<String> = Vec::new();        
        for block in &self.blocks {
            let status_str = match block.status {
                Status::Used => "used",
                Status::Free => "free"
            };
            blocks_stringify.push(format!("{}:{}:{}", block.data_address, block.size, status_str));
        }
        blocks_stringify
    }
    

    // Get allocator's class indexes to alloc memory that is next to a power of 2 class - O(1) complexity
    pub fn get_class_index(&self, request_size: i32) -> Option<usize> {
        self.class.sizes.iter().position(|&c| c >= request_size) // If i want to alloc 57, it is
        // indexed into a 64 class.
    }

    // Set block's class reference inside allocator's registry
    pub fn set_class_in_box(&mut self, block_addr: i32, class_index: usize) {
        if let Some(matched_block_idx) = self.blocks.iter().
            position(|b| b.data_address == block_addr) {
            let matched_block = &mut self.blocks[matched_block_idx];
            matched_block.class = Some(self.class.sizes[class_index]);
        }     
    }

    // Allocate memory that is free and suitable
    pub fn alloc_by_idx(&mut self, request_size: i32, block_idx: Option<usize>) -> Result<i32, &'static str> {
        // Base case, more allocated size that default owned
        if request_size > DEFAULT_HEAP_SIZE {
            return Err("OOM")
        }

        if let Some(idx) = block_idx {
            // Resolve class size
            let class_idx = match self.get_class_index(request_size) {
                Some(c) => c,
                None    => return Err("OOM"),
            };
            
            let class_val = self.class.sizes[class_idx];
            let wasted_space = class_val - request_size;
            self.internal_frag += wasted_space;
            //calculate the total footprint of the newly allocated block (payload + tags) - taking
            //into account that if request_size = 10 and is included in class 16, the size is 16 not 10
            //let used_block_footprint = class_val + OVERHEAD; - KNUTH original


            let block = &mut self.blocks[idx]; // It's gonna be updated
            let original_data_addr = block.data_address;
            let original_data_size = block.size;
            //let original_data_size = class_val;
            
            if original_data_size >= class_val + MIN_SPLIT {
                // split initial free block into used + free blocks
                block.size     = class_val;
                block.status   = Status::Used;
                block.class    = Some(class_val);
                block.req_size = request_size;
                self.class.alloc_count[class_idx] += 1;
                // If the class previously had free blocks recorded, decrement by 1
                if self.class.free_count[class_idx] > 0 {
                    self.class.free_count[class_idx] -= 1;
                }
                                
                // create the new free block due to used block appearance
                let new_free_addr = original_data_addr + class_val; // KNUTH original goes
                // used_block_footprint
                let new_free_payload_size = original_data_size - class_val; // KNUTH original goes
                // used_block_footprint

                let free_block = MemoryBlock {
                    data_address: new_free_addr,
                    size: new_free_payload_size,
                    status: Status::Free,
                    class: None,
                    req_size: 0,
                };

                // add new free block into block registry, next to used block already registered
                self.blocks.insert(idx + 1, free_block);
            } else {
                // whole initial free block is gonna be used + it'll contains overflow
                block.size     = class_val;
                block.status   = Status::Used;
                block.class    = Some(class_val);
                block.req_size = request_size;
                self.class.alloc_count[class_idx] += 1;
                if self.class.free_count[class_idx] > 0 {
                    self.class.free_count[class_idx] -= 1;
                }
            }
            
            Ok(original_data_addr)
        } else {
            Err("OOM")
        }
    }

    // Frees used blocks of memory from data_adress
    pub fn free(&mut self, received_class: i32) -> Result<&'static str, &'static str> {
        let mut found = false;
        
        for block in self.blocks.iter_mut() {
            if block.status == Status::Used && block.data_address == received_class {
                block.status = Status::Free;
                found = true;
                // If removed alloc block, substract internal frag from that block
                if let Some(c) = block.class {
                    if self.internal_frag > 0 {
                        self.internal_frag -= c - block.req_size;
                    }
                }
                break;
            }
        }

        if found {
            if let Some(idx) = self.class.sizes.iter().position(|&s| s == received_class) {
                self.class.free_count[idx]  += 1;
                self.class.alloc_count[idx] -= 1;
            }
            Ok("OK")
        } else {
            Err("BAD")
        }
    }

    // Prints free blocks
    pub fn print_free_block(&self) -> Vec<String> {
        let mut free_blocks_stringify: Vec<String> = Vec::new();
        for block in &self.blocks {
            if block.status == Status::Free {
                free_blocks_stringify.push(format!("{}:{}", block.data_address, block.size));
            }
        }
        free_blocks_stringify
    }

    // Coalesce consecutive free blocks innto a bigger one
    pub fn coalesce(&mut self) {
        // Base case
        if self.blocks.len() < 2 {
            return;
        }

        let mut i = 0; // index for block registry iteration
        while i < self.blocks.len() - 1 {
            let current_block_is_free = self.blocks[i].status == Status::Free;
            let next_block_is_free = self.blocks[i + 1].status == Status::Free;

            if current_block_is_free && next_block_is_free {
                // Group payload size from adjacent blocks that are coalescing + OVERHEAD
                let combined_payload = self.blocks[i].size + self.blocks[i+1].size + OVERHEAD;
                // set new payload size to current coalesced block
                self.blocks[i].size = combined_payload;
                // remove adjacent block
                self.blocks.remove(i + 1);

                // No i increment needed because we removed one, so it will compare with the newly
                // expanded
            } else {
                i += 1;
            }
        }
    }

    // Looks for the previous block
    pub fn prev(&self, check_from_addr: i32) -> String {
        let block_idx = match self.blocks.iter()
            .position(|b| b.data_address == check_from_addr) {
            Some(idx) => idx,
            None             => return "BAD".to_string(),
        };

        // If we are on the first block, there is noone behind
        if block_idx == 0 {
            return "NONE".to_string()
        }

        let previous_block = &self.blocks[block_idx - 1]; // O(1)
        let formatted_status = match previous_block.status {
            Status::Used => "used",
            Status::Free => "free"
        };
        
        format!("{}:{}:{}", previous_block.data_address, previous_block.size, formatted_status)
    }

    // Count total blocks in memory at this time
    pub fn count(&self) -> String {
        self.blocks.iter().count().to_string()
    }

    // Prints info related to defined classes
    pub fn stats(&self) -> Vec<String> {
        let mut info_classes_stringify: Vec<String> = Vec::new();

        for i in 0..self.class.sizes.len() {
            let class_size = self.class.sizes[i];
            let allocators = self.class.alloc_count[i];
            let frees      = self.class.free_count[i];

            info_classes_stringify.push(format!("{}:alloc={}:free={}", class_size, allocators, frees));
        }
        info_classes_stringify
    }

    pub fn report(&self) -> String {
        let mut total_used = 0;
        let mut total_free = 0;
        let mut free_blocks_count = 0;
        let mut largest_free = 0;

        for block in &self.blocks {
            match block.status {
                Status::Free => {
                    total_free += block.size;
                    free_blocks_count += 1;
                    if block.size > largest_free {
                        largest_free = block.size;
                    }
                }
                Status::Used => {
                    total_used += block.size;
                }
            }
        }

        let external_frag = if total_free > 0 {
            1.0 - (largest_free as f64 / total_free as f64)
        } else {
            0.0
        };

        format!("used={} free={} free_blocks={} largest_free={} internal_frag={} external_frag={:.4}",
            total_used, total_free, free_blocks_count, largest_free, self.internal_frag, external_frag)
    }
}

impl ThreadCacheManager {
    // New instance
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, thread_id: i32, class: i32) -> String {
        let thread = self.threads.entry(thread_id).or_default();
        let local_count = thread.entry(class).or_insert(0); // Default
        // value as 0 if there is no local block count
        if *local_count > 0 {
            *local_count -= 1;
            "local".to_string()
        } else { // thread local cache for class is empty
            // Cache MISS: Batch 4 blocks from central cache
            *local_count = BATCH - 1;
            *self.central.entry(class).or_insert(0) += BATCH;
            "central".to_string()
        }
    }

    pub fn free(&mut self, thread_id: i32, class: i32) -> String {
        let thread = self.threads.entry(thread_id).or_default();
        let local_count = thread.entry(class).or_insert(0);
        let central_available = self.central.entry(class).or_insert(0);

        if *local_count < CACHE_MAX {
            *local_count += 1;
            if *central_available > 0 {
                *central_available -= 1;
            }
            "local".to_string()
        } else {
            *local_count -= HALF;
            *central_available += HALF;
            "flush".to_string()
        }
    }

    pub fn stats(&self, class: i32) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(cache) = self.threads.get(&class) {
            for (&class, &count) in cache {
                if count > 0 {
                    out.push(format!("class={}:{}", class, count));
                }
            }
        }
        out
    }
}

impl AllocatorEngine {
    pub fn alloc(&mut self, size: Option<i32>, name: Option<String>, thread_id: Option<i32>, class: Option<i32>) -> String {
        match self {
            AllocatorEngine::Knuth(ctl) => {
                if let Some(size) = size {
                    match ctl.alloc(size) {
                        //Ok(class)         => format!("class={}", class), - Pure knuth with class
                        //visualization
                        Ok(addr)          => addr.to_string(),
                        Err(err) => err.to_string()
                    }
                } else {
                    "MISSING_SIZE_PARAM".to_string()
                }
            },
            AllocatorEngine::Buddy(ctl) => {
                if let Some(size) = size {
                    match ctl.alloc(size as usize)  {
                        Ok(addr) => addr.to_string(),
                        Err(err) => err.to_string()
                    }
                } else {
                    "MISSING_SIZE_PARAM".to_string()
                }
            },   
            AllocatorEngine::Slab(caches) => {
                if let Some(cache_name) = name {
                    if let Some(allocator) = caches.get_mut(&cache_name) {
                        match allocator.alloc(cache_name) {
                            Ok(slab_slot) => slab_slot,
                            Err(err) => err.to_string(),

                        }
                    } else {
                        "BAD".to_string()
                    }
                } else {
                    "MISSING_NAME_PARAM".to_string()
                }
            },
            AllocatorEngine::ThreadLocalCache(ctl) => {
                if let (Some(thread_id), Some(class)) = (thread_id, class) {
                    ctl.alloc(thread_id, class)
                } else {
                    "BAD".to_string()
                }
            }
        }
    }


    pub fn free(&mut self, number: Option<i32>, name: Option<String>, slab: Option<usize>, slot: Option<usize>, thread_id: Option<i32>, class: Option<i32>) -> String {
        match self {
            AllocatorEngine::Knuth(ctl) => {
                if let Some(number) = number {
                    match ctl.allocator.free(number) {
                        Ok(str_res) => {
                            ctl.allocator.coalesce();
                            str_res.to_string()
                        },
                        Err(err) => err.to_string()
                    }
                } else {
                    "MISSING_NUMBER_PARAM".to_string()
                }
            },
            AllocatorEngine::Buddy(ctl) => {
                if let Some(number) = number {
                    match ctl.free(number) {
                        Ok(str_res) => str_res.to_string(),
                        Err(err) => err.to_string()
                    }
                } else {
                    "MISSING_NUMBER_PARAM".to_string()
                }
            },
            AllocatorEngine::Slab(caches) => {
                if let (Some(name), Some(slab), Some(slot)) = (name, slab, slot) {
                    if let Some(allocator) = caches.get_mut(&name) {
                        allocator.free(name, slab, slot)
                    } else {
                        "BAD".to_string()
                    }
                } else {
                    "MISSING_REQUIRED_PARAMS".to_string()
                }
            },
            AllocatorEngine::ThreadLocalCache(ctl) => {
                if let (Some(thread_id), Some(class)) = (thread_id, class) {
                    ctl.free(thread_id, class)
                } else {
                    "BAD".to_string()
                }
            }
        }
    }

    pub fn free_list(&self, order: usize) -> String {
        match self {
            AllocatorEngine::Knuth(ctl) => ctl.allocator.print_free_block().join("\n"),
            AllocatorEngine::Buddy(ctl) => ctl.free_lists(order),
            AllocatorEngine::Slab(_) => "NOT_SUPPORTED".to_string(),
            AllocatorEngine::ThreadLocalCache(_) => "NOT_SUPPORTED".to_string()
        }
    }

    pub fn stats(&self, name: Option<String>, class: Option<i32>) -> String {
        match self {
            AllocatorEngine::Knuth(ctl) => ctl.allocator.stats().join("\n"),
            AllocatorEngine::Buddy(_) => "NOT_SUPPORTED".to_string(),
            AllocatorEngine::Slab(caches) => {
                if let Some(name) = name {
                    if let Some(allocator) = caches.get(&name) {
                        allocator.stats(name)
                    } else {
                        "BAD".to_string()
                    }
                } else {
                    "MISSING_NAME_PARAM".to_string()
                }
            },
            AllocatorEngine::ThreadLocalCache(ctl) => {
                if let Some(class) = class {
                    ctl.stats(class).join("\n")
                } else {
                    "MISSING_CLASS_PARAM".to_string()
                }
            }
        }
    }
}

pub fn lazy_cotroller_init(
    fit_strat_controller: &mut Option<FitStratController>, 
    _request_size: i32, 
    active_strat: FitStrategies) -> &mut FitStratController {
    // Lazily initialize the controller if no prior INIT instruction was executed
    fit_strat_controller.get_or_insert_with(|| {
        let class_instance = Class::new();
        let allocator = Allocator::init(DEFAULT_HEAP_SIZE, class_instance);
        FitStratController::new(allocator, active_strat)
    })
}

const HEADER_SIZE: i32       = 8; // In this memory allocator, the header size is only 8
const FOOTER_SIZE: i32       = 8;
const MIN_SPLIT: i32         = 16; // Minimum remaining size of unued block memory that determines split.
const OVERHEAD: i32          = 0; // HEADER_SIZE + FOOTER_SIZE if head + foot implemented
const DEFAULT_HEAP_SIZE: i32 = 1024;
const CACHE_MAX: usize       = 5;
const BATCH: usize           = 4;
const HALF: usize            = 3;

fn main() {
    let stdin = io::stdin();
    let mut bump = 0; // Initialize dump controller
    let mut result: Vec<String> = Vec::new(); // Initialize result vector that will contain partial solutions
    let mut engine: Option<AllocatorEngine> = None;
    let allocator_workload = AllocatorsWorkload::init();
    // initialization
    //let mut allocator = Allocator::new(); // Vector that will registry memory blocks
    // in memory
    for line in stdin.lock().lines() {
        let l = line.unwrap();
        if l.is_empty() { continue; }

        // Definition of the type of allocator needed to the requirements received
        result.push(allocator_workload.match_pattern(l.as_str()));

        /*let mut expression = l.split_whitespace(); // Get expression input
        // separated into whitespaces
        let instruction: Option<AllowedInstructions> = expression
            .next()
            .and_then(|s| s.parse().ok());// Get the instruction part parsed as valid enum        
        /*let strategy: Option<FitStrategies> = expression
            .next()
            .and_then(|s| s.parse().ok());
        let active_strat = strategy.unwrap_or(FitStrategies::FIRST);*/ // Uncomment when using Knuth alg
        match instruction {
            Some(AllowedInstructions::INIT) => {
                let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if let Some(size) = number {
                    
                    let class_instance = Class::new();
                    let allocator = Allocator::init(size, class_instance);
                    engine = Some(AllocatorEngine::Knuth(FitStratController::new(allocator, FitStrategies::FIRST)));
                    // Selected Buddy algorithm as Allocator engine for this tests
                    //engine = Some(AllocatorEngine::Buddy(BudyAllocator::new(size as usize)));
                    result.push("OK".to_string());
                } else {
                    println!("Inapropiate value for instruction INSERT, expected: num, got: {:?}", number);
                    return;
                }
            }
            Some(AllowedInstructions::ALLOC) => {
                // Knuth
                let size: Option<i32> = expression.next().and_then(|s| s.trim().parse().ok());
                if let Some(eng) = &mut engine {
                    result.push(eng.alloc(size, None, None, None));
                }
                // BUDDY
                /*let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if let Some(eng) = &mut engine {
                    result.push(eng.alloc(number, None));
                }*/
                // -- SLAB
                /*let cache_name: Option<String> = expression.next().and_then(|s| s.trim().parse().ok());
                if let Some(eng) = &mut engine {
                    result.push(eng.alloc(None, cache_name, None, None));
                }*/
                // THREAD-CACHE
                /*let tid: Option<i32>   = expression.next().and_then(|t| t.parse().ok());
                let class: Option<i32> = expression.next().and_then(|c| c.parse().ok());
                // Lazy init the engine
                let eng = engine.get_or_insert_with(|| {
                    AllocatorEngine::ThreadLocalCache(ThreadCacheManager::new())
                });

                result.push(eng.alloc(None, None, tid, class));*/
                // Decide whether to reuse or bump
                // --- WHEN WE DONT TAKE INTO ACCOUNT FREE BLOCKS AS FIRST
                /*if let Some(addr) = reused_block_addr {
                    // Reused an existing freed block
                    result.push(addr.to_string());
                } else {
                    // No free block fits — fall back to bumping the heap pointer
                    let block_total = val + HEADER_SIZE;

                    if bump + block_total > heap_size {
                        result.push("OOM".to_string());
                    } else {
                        let data_addr = bump + HEADER_SIZE;
            
                        result.push(data_addr.to_string());

                        let new_mem_block = MemoryBlock {
                            data_address: data_addr,
                            size: val,
                            status: Status::Used,
                        };
                        allocator.blocks.push(new_mem_block);

                        bump += block_total;
                    }
                }
                } else {
                    println!("Inappropriate value for instruction ALLOC");
                    return;
                }*/
            }
            Some(AllowedInstructions::USED) => {
                let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if number.is_some() {
                    println!("Unexpected value for instruction USED: got: {:?}", number);
                    return;
                } else {
                    result.push(bump.to_string()); // Current use of memory size
                }
            }
            Some(AllowedInstructions::RESET) => {
                let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if number.is_some() {
                    println!("Unexpected value for instruction RESET: got: {:?}", number);
                    return;
                } else {
                    bump = 0; // Rest memory use of size
                    result.push("OK".to_string());
                }
            }
            Some(AllowedInstructions::FREE) => {
                // Knuth
                let addr: Option<i32> = expression.next().and_then(|s| s.trim().parse().ok());
                if let Some(eng) = &mut engine {
                    result.push(eng.free(addr, None, None, None, None, None));
                }
                // BUDYY
                /*let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if let Some(eng) = &mut engine {
                    result.push(eng.free(number, None, None, None));
                }*/
                // SLAB
                /*let cache_name: Option<String> = expression.next().and_then(|s| s.trim().parse().ok());
                let sid: Option<usize> = expression.next().and_then(|n| n.parse().ok());
                let slot: Option<usize> = expression.next().and_then(|n| n.parse().ok());
                if let Some(eng) = &mut engine {
                    result.push(eng.free(None, cache_name, sid, slot, None, None))
                }*/
                // THREAD-CACHE
                /*let tid: Option<i32> = expression.next().and_then(|t| t.parse().ok());
                let class: Option<i32> = expression.next().and_then(|c| c.parse().ok());
                let eng = engine.get_or_insert_with(|| {
                    AllocatorEngine::ThreadLocalCache(ThreadCacheManager::new())
                });
                result.push(eng.free(None, None, None, None, tid, class));*/
                
                // Content from previous tests may help in the future
                /*if let Some(val) = number {
                    let mut found: bool = false;
                    for block in allocator.blocks.iter_mut() {
                        if block.data_address == val && block.status == Status::Used {
                            block.status = Status::Free; // Change status of matched block
                            found = true; // We found the block to free, now we can return OK
                            break; // Stop after finding the one
                        }
                    }
                    if found {
                        result.push("OK".to_string());
                    } else {
                        result.push("BAD".to_string());
                    }
                } else {
                    println!("Unexpected value for instruction FREE: got: {:?}", number);
                    return;
                }*/
            }
            Some(AllowedInstructions::BLOCKS) => {
                if let Some(AllocatorEngine::Knuth(ctl)) = &engine {
                    result.extend(ctl.allocator.print_blocks());
                }
            }
            Some(AllowedInstructions::FREELIST) => {
                let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if let (Some(eng), Some(order)) = (&engine, number) {
                    result.push(eng.free_list(order as usize));
                }
            }
            Some(AllowedInstructions::PREV) => {
                let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if let (Some(AllocatorEngine::Knuth(ctl)), Some(addr)) = (&mut engine, number) {
                    result.push(ctl.allocator.prev(addr));
                } else {
                    println!("Inapropiate value for instruction INSERT, expected: num, got: {:?}", number);
                    return;
                }

            }
            Some(AllowedInstructions::COUNT) => {
                if let Some(AllocatorEngine::Knuth(ctl)) = &engine {
                    result.push(ctl.allocator.count());
                }
            }
            Some(AllowedInstructions::STATS) => {
                // KNUTH
                /*let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if let Some(AllocatorEngine::Knuth(ctl)) = &engine {
                    result.push(ctl.allocator.stats());
                }*/
                // SLAB
                /*let cache_name: Option<String> = expression.next().and_then(|s| s.trim().parse().ok());
                if let (Some(eng), Some(cache_name)) = (&engine, cache_name) {
                    result.push(eng.stats(cache_name))
                }*/
                // THREAD-CACHE
                let tid = expression.next().and_then(|t| t.parse().ok());
                if let Some(eng) = &engine {
                    result.push(eng.stats(None, tid))
                } 
            }
            Some(AllowedInstructions::ORDER) => {
                let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                if let (Some(AllocatorEngine::Buddy(buddy)), Some(order)) = (&engine, number) {
                    result.push(buddy.order(order as usize));
                }
            }
            Some(AllowedInstructions::BUDDY) => {
                let number: Option<i32> = if let Some(s) = expression.next() {
                    s.trim().parse::<i32>().ok()
                } else {
                    None
                }; // Get the Optional number part, i might not get it so thats why i need an option

                let order_param: Option<usize> = expression.next().and_then(|o| o.parse().ok());
                if let (Some(AllocatorEngine::Buddy(buddy)), Some(addr), Some(order)) = (&engine, number, order_param) {
                    result.push(buddy.buddy(addr, order));
                }
            }
            Some(AllowedInstructions::CACHE_CREATE) => {
                let name: Option<String> = expression.next().and_then(|s| s.trim().parse().ok());            
                let obj_size: Option<usize> = expression.next().and_then(|s| s.parse().ok());
                let slab_obj_count: Option<usize> = expression.next().and_then(|s| s.parse().ok());

                if let (Some(cache_name), Some(object_size), Some(slab_object_count)) = (name, obj_size, slab_obj_count) {
                    let caches = match &mut engine {
                        Some(AllocatorEngine::Slab(map)) => map,
                        _ => {
                            engine = Some(AllocatorEngine::Slab(HashMap::new()));
                            if let Some(AllocatorEngine::Slab(map)) = &mut engine {
                                map
                            } else {
                                unreachable!()
                            }
                        }
                    };

                    let new_allocator = SlabAllocator::new(cache_name.clone(), object_size, slab_object_count);
                    caches.insert(cache_name, new_allocator);
                    result.push("OK".to_string())
                } else {
                    println!("Invalid arguments for CACHE_CREATE instruction");
                    return;
                }
            }
            Some(AllowedInstructions::REPORT) => {
                if let Some(AllocatorEngine::Knuth(ctl)) = &engine {
                    result.push(ctl.allocator.report());
                }
            }
            None => println!("Invalid or missing instruction"),
        }*/
    }
    for value in result.iter() {
        println!("{}", value);
    }
}
