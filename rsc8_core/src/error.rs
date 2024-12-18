pub enum InstructionError {
    UnknownOpcode(u16),
}

impl core::fmt::Debug for InstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InstructionError::UnknownOpcode(opcode) => write!(f, "UnknownOpcode({:04x})", opcode),
        }
    }
}

impl core::fmt::Display for InstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl core::error::Error for InstructionError {}
