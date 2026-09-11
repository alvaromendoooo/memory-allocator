use std::io::{self, BufRead};
use std::str::FromStr;

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

// Definition of memory block components
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub data_address: i32,
    pub size: i32,
    pub status: Status,
    pub class: Option<i32>
}

// Defiition of Classes for allocating memory
pub struct Class {
    pub sizes: Vec<i32>,
    pub alloc_count: Vec<i32>,
    pub free_count: Vec<i32>,
}

// Definition of allocator instance, contolling memory block registry
pub struct Allocator {
    pub blocks: Vec<MemoryBlock>,
    pub class: Class,
}

// Definition of memory strategy allocator manager - Controls ALLOC depending of the strat
pub struct FitStratController {
    pub allocator: Allocator,
    pub strat: FitStrategies
}

// Mapper that converts input str into enum for match iteration control
impl FromStr for AllowedInstructions {     

    type Err = ParseInstructionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "INIT" => Ok(AllowedInstructions::INIT),
            "ALLOC"    => Ok(AllowedInstructions::ALLOC),
            "USED"     => Ok(AllowedInstructions::USED),
            "RESET"    => Ok(AllowedInstructions::RESET),
            "FREE"     => Ok(AllowedInstructions::FREE),
            "BLOCKS"   => Ok(AllowedInstructions::BLOCKS),
            "FREELIST" => Ok(AllowedInstructions::FREELIST),
            "PREV"     => Ok(AllowedInstructions::PREV),
            "COUNT"    => Ok(AllowedInstructions::COUNT),
            "STATS"    => Ok(AllowedInstructions::STATS),
            _          => Err(ParseInstructionError),
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
        }
    }

    // Create a new instance of Allocator with free data
    pub fn init(heap_size: i32, hdr: i32, ftr: i32, class_instance: Class) -> Self {
        let initial_free_block = MemoryBlock {
            data_address: hdr,
            size: heap_size - hdr - ftr,
            status: Status::Free,
            class: None
        };

        Self {
            blocks: vec![initial_free_block],
            class: class_instance,
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
            return Err("TOO_LARGE")
        }

        if let Some(idx) = block_idx {
            // Resolve class size
            let class_idx = match self.get_class_index(request_size) {
                Some(c) => c,
                None    => return Err("TOO_LARGE"),
            };
            
            let class_val = self.class.sizes[class_idx];
            //calculate the total footprint of the newly allocated block (payload + tags) - taking
            //into account that if request_size = 10 and is included in class 16, the size is 16 not 10
            let used_block_footprint = class_val + OVERHEAD;


            let block = &mut self.blocks[idx]; // It's gonna be updated
            let original_data_addr = block.data_address;
            let original_data_size = block.size;
            
            if original_data_size >= used_block_footprint + MIN_SPLIT {
                // split initial free block into used + free blocks
                block.size   = request_size;
                block.status = Status::Used;
                block.class  = Some(class_val);
                self.class.alloc_count[class_idx] += 1;
                // If the class previously had free blocks recorded, decrement by 1
                if self.class.free_count[class_idx] > 0 {
                    self.class.free_count[class_idx] -= 1;
                }
                                
                // create the new free block due to used block appearance
                let new_free_addr = original_data_addr + used_block_footprint;
                let new_free_payload_size = original_data_size - used_block_footprint;

                let free_block = MemoryBlock {
                    data_address: new_free_addr,
                    size: new_free_payload_size,
                    status: Status::Free,
                    class: None
                };

                // add new free block into block registry, next to used block already registered
                self.blocks.insert(idx + 1, free_block);
            } else {
                // whole initial free block is gonna be used + it'll contains overflow
                block.size = class_val;
                block.status = Status::Used;
                block.class  = Some(class_val);
                self.class.alloc_count[class_idx] += 1;
                if self.class.free_count[class_idx] > 0 {
                    self.class.free_count[class_idx] -= 1;
                }
            }
            
            Ok(class_val)
        } else {
            Err("OOM")
        }
    }

    // Frees used blocks of memory from data_adress
    pub fn free(&mut self, received_class: i32) -> Result<&'static str, &'static str> {
        let mut found = false;
        
        for block in self.blocks.iter_mut() {
            if block.status == Status::Used && block.class == Some(received_class) {
                block.status = Status::Free;
                found = true;
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
}

pub fn lazy_cotroller_init(
    fit_strat_controller: &mut Option<FitStratController>, 
    _request_size: i32, 
    active_strat: FitStrategies) -> &mut FitStratController {
    // Lazily initialize the controller if no prior INIT instruction was executed
    fit_strat_controller.get_or_insert_with(|| {
        let class_instance = Class::new();
        let allocator = Allocator::init(DEFAULT_HEAP_SIZE, HEADER_SIZE, FOOTER_SIZE, class_instance);
        FitStratController::new(allocator, active_strat)
    })
}

const HEADER_SIZE: i32       = 8; // In this memory allocator, the header size is only 8
const FOOTER_SIZE: i32       = 8;
const MIN_SPLIT: i32         = 16; // Minimum remaining size of unued block memory that determines split.
const OVERHEAD: i32          = HEADER_SIZE + FOOTER_SIZE;
const DEFAULT_HEAP_SIZE: i32 = 1024;

fn main() {
    let stdin = io::stdin();
    let mut bump = 0; // Initialize dump controller
    let mut result: Vec<String> = Vec::new(); // Initialize result vector that will contain partial solutions
    let mut fit_strat_controller: Option<FitStratController> = None; // Wrapped in Option to safely manage
    // initialization
    //let mut allocator = Allocator::new(); // Vector that will registry memory blocks
    // in memory
    for line in stdin.lock().lines() {
        let l = line.unwrap();
        if l.is_empty() { continue; }
        let mut expression = l.split_whitespace(); // Get expression input
        // separated into whitespaces
        let instruction: Option<AllowedInstructions> = expression
            .next()
            .and_then(|s| s.parse().ok());// Get the instruction part parsed as valid enum
        let number: Option<i32> = if let Some(s) = expression.next() {
            s.trim().parse::<i32>().ok()
        } else {
            None
        }; // Get the Optional number part, i might not get it so thats why i need an option
        
        let strategy: Option<FitStrategies> = expression
            .next()
            .and_then(|s| s.parse().ok());
        let active_strat = strategy.unwrap_or(FitStrategies::FIRST); 
        match instruction {
            Some(AllowedInstructions::INIT) => {
                if let Some(size) = number {
                    
                    let class_instance = Class::new();
                    let allocator = Allocator::init(size, HEADER_SIZE, FOOTER_SIZE, class_instance);
                    fit_strat_controller = Some(FitStratController::new(allocator, active_strat));
                    result.push("OK".to_string());
                } else {
                    println!("Inapropiate value for instruction INSERT, expected: num, got: {:?}", number);
                    return;
                }
            }
            Some(AllowedInstructions::ALLOC) => {
                // Look for an existing freed block that fits
                if let Some(size) = number {
                    let ctl = lazy_cotroller_init(&mut fit_strat_controller, size, active_strat);
                    match ctl.alloc(size) {
                        Ok(class) => result.push(format!("class={}", class)),
                        Err(err) => result.push(err.to_string()),
                    }
                }
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
                }*/ else {
                    println!("Inappropriate value for instruction ALLOC, expected: num, got: {:?}", number);
                    return;
                }
            }
            Some(AllowedInstructions::USED) => {
                if number.is_some() {
                    println!("Unexpected value for instruction USED: got: {:?}", number);
                    return;
                } else {
                    result.push(bump.to_string()); // Current use of memory size
                }
            }
            Some(AllowedInstructions::RESET) => {
                if number.is_some() {
                    println!("Unexpected value for instruction RESET: got: {:?}", number);
                    return;
                } else {
                    bump = 0; // Rest memory use of size
                    result.push("OK".to_string());
                }
            }
            Some(AllowedInstructions::FREE) => {
                if let Some(data_address) = number {
                    let ctl = lazy_cotroller_init(&mut fit_strat_controller, DEFAULT_HEAP_SIZE, active_strat);
                    match ctl.allocator.free(data_address) {
                        Ok(str) => {
                            result.push(str.to_string());
                            ctl.allocator.coalesce();
                        },
                        Err(err) => result.push(err.to_string()),
                    }
                } 
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
                if let Some(ctl) = &fit_strat_controller {
                    result.extend(ctl.allocator.print_blocks());
                }
            }
            Some(AllowedInstructions::FREELIST) => {
                if let Some(ctl) = &fit_strat_controller {
                    result.extend(ctl.allocator.print_free_block());
                }
            }
            Some(AllowedInstructions::PREV) => {
                if let (Some(ctl), Some(addr)) = (&mut fit_strat_controller, number) {
                    result.push(ctl.allocator.prev(addr));
                } else {
                    println!("Inapropiate value for instruction INSERT, expected: num, got: {:?}", number);
                    return;
                }

            }
            Some(AllowedInstructions::COUNT) => {
                if let Some(ctl) = &fit_strat_controller {
                    result.push(ctl.allocator.count());
                }
            }
            Some(AllowedInstructions::STATS) => {
                let ctl = lazy_cotroller_init(&mut fit_strat_controller, DEFAULT_HEAP_SIZE, active_strat);
                result.extend(ctl.allocator.stats());
            }
            None => println!("Invalid or missing instruction"),
        }
    }
    for value in result.iter() {
        println!("{}", value);
    }
}
