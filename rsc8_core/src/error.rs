#[derive(PartialEq, Eq)]
pub enum InstructionError {
    UnknownOpcode(u16),
    RomTooLarge { rom_size: usize, max_size: usize },
    ProgramCounterOutOfBounds(u16),
    ProgramCounterOverflow(u16),
    ProgramCounterUnderflow(u16),
    StackOverflow,
    StackUnderflow,
    MemoryOutOfBounds(usize),
    InvalidKeyIndex(u8),
}

impl core::fmt::Debug for InstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InstructionError::UnknownOpcode(opcode) => write!(f, "UnknownOpcode({opcode:04x})"),
            InstructionError::RomTooLarge { rom_size, max_size } => {
                write!(f, "RomTooLarge(rom_size={rom_size}, max_size={max_size})")
            }
            InstructionError::ProgramCounterOutOfBounds(program_counter) => {
                write!(f, "ProgramCounterOutOfBounds(pc=0x{program_counter:04x})")
            }
            InstructionError::ProgramCounterOverflow(program_counter) => {
                write!(f, "ProgramCounterOverflow(pc=0x{program_counter:04x})")
            }
            InstructionError::ProgramCounterUnderflow(program_counter) => {
                write!(f, "ProgramCounterUnderflow(pc=0x{program_counter:04x})")
            }
            InstructionError::StackOverflow => write!(f, "StackOverflow"),
            InstructionError::StackUnderflow => write!(f, "StackUnderflow"),
            InstructionError::MemoryOutOfBounds(address) => {
                write!(f, "MemoryOutOfBounds(address={address})")
            }
            InstructionError::InvalidKeyIndex(key) => {
                write!(f, "InvalidKeyIndex(key={key})")
            }
        }
    }
}

impl core::fmt::Display for InstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for InstructionError {}
