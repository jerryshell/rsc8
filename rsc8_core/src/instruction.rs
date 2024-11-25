pub enum Instruction {
    Ins00E0,
    Ins00EE,
    Ins1NNN(u16),
    Ins2NNN(u16),
    Ins3XNN(u8, u8),
    Ins4XNN(u8, u8),
    Ins5XY0(u8, u8),
    Ins6XNN(u8, u8),
    Ins7XNN(u8, u8),
    Ins8XY0(u8, u8),
    Ins8XY1(u8, u8),
    Ins8XY2(u8, u8),
    Ins8XY3(u8, u8),
    Ins8XY4(u8, u8),
    Ins8XY5(u8, u8),
    Ins8XY6(u8, u8),
    Ins8XY7(u8, u8),
    Ins8XYE(u8, u8),
    Ins9XY0(u8, u8),
    InsANNN(u16),
    InsBNNN(u16),
    InsCXNN(u8, u8),
    InsDXYN(u8, u8, u8),
    InsEX9E(u8),
    InsEXA1(u8),
    InsFX07(u8),
    InsFX0A(u8),
    InsFX15(u8),
    InsFX18(u8),
    InsFX1E(u8),
    InsFX29(u8),
    InsFX33(u8),
    InsFX55(u8),
    InsFX65(u8),
}

impl From<u16> for Instruction {
    fn from(opcode: u16) -> Self {
        let nibble1 = ((opcode & 0xF000) >> 12) as u8;
        let nibble2 = ((opcode & 0x0F00) >> 8) as u8;
        let nibble3 = ((opcode & 0x00F0) >> 4) as u8;
        let nibble4 = (opcode & 0x000F) as u8;
        match nibble1 {
            0x0 => match nibble4 {
                // 00E0: Clear screen
                0x0 => Instruction::Ins00E0,
                // 00EE: Return from subroutine
                0xE => Instruction::Ins00EE,
                _ => panic!("Unknown opcode: {:04x}", opcode),
            },
            // 1NNN: Jump to NNN
            0x1 => Instruction::Ins1NNN(opcode & 0x0FFF),
            // 2NNN: Call subroutine at NNN
            0x2 => Instruction::Ins2NNN(opcode & 0x0FFF),
            // 3XNN: Skip next instruction if VX == NN
            0x3 => Instruction::Ins3XNN(nibble2, (opcode & 0x00FF) as u8),
            // 4XNN: Skip next instruction if VX != NN
            0x4 => Instruction::Ins4XNN(nibble2, (opcode & 0x00FF) as u8),
            // 5XY0: Skip next instruction if VX == VY
            0x5 => Instruction::Ins5XY0(nibble2, nibble3),
            // 6XNN: VX = NN
            0x6 => Instruction::Ins6XNN(nibble2, (opcode & 0x00FF) as u8),
            // 7XNN: VX += NN
            0x7 => Instruction::Ins7XNN(nibble2, (opcode & 0x00FF) as u8),
            0x8 => match nibble4 {
                // 8XY0: VX = VY
                0x0 => Instruction::Ins8XY0(nibble2, nibble3),
                // 8XY1: VX = (VX |= VY)
                0x1 => Instruction::Ins8XY1(nibble2, nibble3),
                // 8XY2: VX = (VX &= VY)
                0x2 => Instruction::Ins8XY2(nibble2, nibble3),
                // 8XY3: VX = (VX ^= VY)
                0x3 => Instruction::Ins8XY3(nibble2, nibble3),
                // 8XY4: VX += VY
                0x4 => Instruction::Ins8XY4(nibble2, nibble3),
                // 8XY5: VX -= VY
                0x5 => Instruction::Ins8XY5(nibble2, nibble3),
                // 8XY6: VX = VY, VX >>= 1
                0x6 => Instruction::Ins8XY6(nibble2, nibble3),
                // 8XY7: VX = VY - VX
                0x7 => Instruction::Ins8XY7(nibble2, nibble3),
                // 8XYE: VX = VY, VX <<= 1
                0xE => Instruction::Ins8XYE(nibble2, nibble3),
                _ => panic!("Unknown opcode: {:04x}", opcode),
            },
            // 9XY0: Skip next instruction if VX != VY
            0x9 => Instruction::Ins9XY0(nibble2, nibble3),
            // ANNN: I = NNN
            0xA => Instruction::InsANNN(opcode & 0x0FFF),
            // BNNN: Jump to NNN + V0
            0xB => Instruction::InsBNNN(opcode & 0x0FFF),
            // CXNN: VX = rand() & NN
            0xC => Instruction::InsCXNN(nibble2, (opcode & 0x00FF) as u8),
            // DXYN: Draw (8 width * N height) sprite at (VX, VY)
            0xD => Instruction::InsDXYN(nibble2, nibble3, nibble4),
            0xE => match nibble3 {
                // EX9E: Skip next instruction if key[VX] is pressed
                0x9 => Instruction::InsEX9E(nibble2),
                // EXA1: Skip next instruction if key[VX] is not pressed
                0xA => Instruction::InsEXA1(nibble2),
                _ => panic!("Unknown opcode: {:04x}", opcode),
            },
            0xF => match nibble3 {
                0x0 => match nibble4 {
                    // FX07: VX = delay_timer
                    0x7 => Instruction::InsFX07(nibble2),
                    // FX0A: Wait for key press, store key in VX
                    0xA => Instruction::InsFX0A(nibble2),
                    _ => panic!("Unknown opcode: {:04x}", opcode),
                },
                0x1 => match nibble4 {
                    // FX15: delay_timer = VX
                    0x5 => Instruction::InsFX15(nibble2),
                    // FX18: sound_timer = VX
                    0x8 => Instruction::InsFX18(nibble2),
                    // FX1E: I += VX
                    0xE => Instruction::InsFX1E(nibble2),
                    _ => panic!("Unknown opcode: {:04x}", opcode),
                },
                // FX29: I = address of font in VX
                0x2 => Instruction::InsFX29(nibble2),
                // FX33: Binary-coded decimal representation of VX
                0x3 => Instruction::InsFX33(nibble2),
                // FX55: Store V0 to VX in memory starting at I
                0x5 => Instruction::InsFX55(nibble2),
                // FX65: Fill V0 to VX with values from memory starting at I
                0x6 => Instruction::InsFX65(nibble2),
                _ => panic!("Unknown opcode: {:04x}", opcode),
            },
            _ => panic!("Unknown opcode: {:04x}", opcode),
        }
    }
}
