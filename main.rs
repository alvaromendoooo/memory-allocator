use std::io::{self, BufRead};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
enum AllowedInstructions {
    INIT,
    ALLOC,
    USED,
    RESET,
    FREE,
    BLOCKS,
    FREELIST,
}

#[derive(Debug, Clone, PartialEq)]
enum Status {
    Used,
    Free
}

// Concret struct to handle parsing str->enum errors
#[derive(Debug)]
struct ParseInstructionError;

// Definition of memory block components
#[derive(Debug, Clone)]
struct MemoryBlock {
    data_address: i32,
    size: i32,
    status: Status,
}

struct Allocator {
    blocks: Vec<MemoryBlock>,
}

impl FromStr for AllowedInstructions { // Mapper that converts input str into enum for match
    // iteration control
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
            _          => Err(ParseInstructionError),
        }
    }
}

impl Allocator {
    // New instace
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
        }
    }

    // Create a new instance of Allocator with free data
    pub fn init(heap_size: i32, hdr: i32) -> Self {
        let initial_free_block = MemoryBlock {
            data_address: hdr,
            size: heap_size - hdr,
            status: Status::Free
        };

        Self {
            blocks: vec![initial_free_block],
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

    // Allocate memory that is free and suitable
    pub fn alloc(&mut self, request_size: i32) -> Result<i32, &'static str> {
        // Find the index of the first free block large enough
        let block_idx = self.blocks.iter().position(|b| {
            b.status == Status::Free && b.size >= request_size
        });

        if let Some(idx) = block_idx {
            let block = &mut self.blocks[idx]; // It's gonna be updated
            let original_data_addr = block.data_address;
            let original_data_size = block.size;
            let remainder_size = original_data_size - request_size;

            if remainder_size >= MIN_SPLIT {
                // Split initial free block into used + free blocks
                block.size = request_size;
                block.status = Status::Used;

                // Create the new free block due to used block appearance
                let new_free_addr = original_data_addr + request_size;

                let free_block = MemoryBlock {
                    data_address: new_free_addr,
                    size: remainder_size,
                    status: Status::Free
                };

                // Add new free block into block registry, next to used block already registered
                self.blocks.insert(idx + 1, free_block);
            } else {
                // Whole initial free block is gonna be used + it'll contains overflow
                block.status = Status::Used;
            }

            Ok(original_data_addr)
        } else {
            Err("OOM")
        }
    }

    // Frees used blocks of memory from data_adress
    pub fn free(&mut self, received_address: i32) -> Result<&'static str, &'static str> {
        let mut found = false;
        for block in self.blocks.iter_mut() {
            if block.status == Status::Used && block.data_address == received_address {
                block.status = Status::Free;
                found = true;
            }
        }

        if found {
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
}



const HEADER_SIZE: i32 = 8; // In this memory allocator, the header size is only 4
const MIN_SPLIT: i32   = 16; // Minimum remaining size of unued block memory that determines split.

fn main() {
    let stdin = io::stdin();
    let mut bump = 0; // Initialize dump controller
    let mut heap_size: i32 ; // Initialize heap size for allocator
    let mut result: Vec<String> = Vec::new(); // Initialize result vector that will contain partial solutions
    let mut allocator = Allocator::new();
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
        
        match instruction {
            Some(AllowedInstructions::INIT) => {
                if let Some(val) = number {
                    heap_size = val; // Sets heap size value
                    allocator = Allocator::init(heap_size, HEADER_SIZE);
                    result.push("OK".to_string());
                } else {
                    println!("Inapropiate value for instruction INSERT, expected: num, got: {:?}", number);
                    return;
                }
            }
            Some(AllowedInstructions::ALLOC) => {
                // Look for an existing freed block that fits
                if let (alloc, Some(size)) = (&mut allocator, number) {
                    match alloc.alloc(size) {
                        Ok(addr) => result.push(addr.to_string()),
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
                if let (alloc, Some(data_address)) = (&mut allocator, number) {
                    match alloc.free(data_address) {
                        Ok(str) => result.push(str.to_string()),
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
                for block_string in allocator.print_blocks() {
                    result.push(block_string);
                }
            }
            Some(AllowedInstructions::FREELIST) => {
                for free_block in allocator.print_free_block() {
                    result.push(free_block);
                }
            }
            None => println!("Invalid or missing instruction"),
        }
    }
    for value in result.iter() {
        println!("{}", value);
    }
}
