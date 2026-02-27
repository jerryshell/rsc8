use crate::error::InstructionError;

#[derive(Debug, PartialEq, Eq)]
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

impl TryFrom<u16> for Instruction {
    type Error = InstructionError;

    #[inline(always)]
    fn try_from(opcode: u16) -> Result<Instruction, InstructionError> {
        let x = ((opcode >> 8) & 0x000F) as u8;
        let y = ((opcode >> 4) & 0x000F) as u8;
        let n = (opcode & 0x000F) as u8;
        let nn = (opcode & 0x00FF) as u8;
        let nnn = opcode & 0x0FFF;

        match opcode & 0xF000 {
            0x0000 => match opcode {
                // 00E0: Clear screen
                0x00E0 => Ok(Instruction::Ins00E0),
                // 00EE: Return from subroutine
                0x00EE => Ok(Instruction::Ins00EE),
                _ => Err(InstructionError::UnknownOpcode(opcode)),
            },
            // 1NNN: Jump to NNN
            0x1000 => Ok(Instruction::Ins1NNN(nnn)),
            // 2NNN: Call subroutine at NNN
            0x2000 => Ok(Instruction::Ins2NNN(nnn)),
            // 3XNN: Skip next instruction if VX == NN
            0x3000 => Ok(Instruction::Ins3XNN(x, nn)),
            // 4XNN: Skip next instruction if VX != NN
            0x4000 => Ok(Instruction::Ins4XNN(x, nn)),
            // 5XY0: Skip next instruction if VX == VY
            0x5000 => match n {
                0x0 => Ok(Instruction::Ins5XY0(x, y)),
                _ => Err(InstructionError::UnknownOpcode(opcode)),
            },
            // 6XNN: VX = NN
            0x6000 => Ok(Instruction::Ins6XNN(x, nn)),
            // 7XNN: VX += NN
            0x7000 => Ok(Instruction::Ins7XNN(x, nn)),
            0x8000 => match n {
                // 8XY0: VX = VY
                0x0 => Ok(Instruction::Ins8XY0(x, y)),
                // 8XY1: VX |= VY
                0x1 => Ok(Instruction::Ins8XY1(x, y)),
                // 8XY2: VX &= VY
                0x2 => Ok(Instruction::Ins8XY2(x, y)),
                // 8XY3: VX ^= VY
                0x3 => Ok(Instruction::Ins8XY3(x, y)),
                // 8XY4: VX += VY
                0x4 => Ok(Instruction::Ins8XY4(x, y)),
                // 8XY5: VX -= VY
                0x5 => Ok(Instruction::Ins8XY5(x, y)),
                // 8XY6: VX = VY, VX >>= 1
                0x6 => Ok(Instruction::Ins8XY6(x, y)),
                // 8XY7: VX = VY - VX
                0x7 => Ok(Instruction::Ins8XY7(x, y)),
                // 8XYE: VX = VY, VX <<= 1
                0xE => Ok(Instruction::Ins8XYE(x, y)),
                _ => Err(InstructionError::UnknownOpcode(opcode)),
            },
            // 9XY0: Skip next instruction if VX != VY
            0x9000 => match n {
                0x0 => Ok(Instruction::Ins9XY0(x, y)),
                _ => Err(InstructionError::UnknownOpcode(opcode)),
            },
            // ANNN: I = NNN
            0xA000 => Ok(Instruction::InsANNN(nnn)),
            // BNNN: Jump to NNN + V0
            0xB000 => Ok(Instruction::InsBNNN(nnn)),
            // CXNN: VX = rand() & NN
            0xC000 => Ok(Instruction::InsCXNN(x, nn)),
            // DXYN: Draw an 8-pixel-wide, N-byte sprite at (VX, VY)
            0xD000 => Ok(Instruction::InsDXYN(x, y, n)),
            0xE000 => match nn {
                // EX9E: Skip next instruction if keypad[VX] is pressed
                0x9E => Ok(Instruction::InsEX9E(x)),
                // EXA1: Skip next instruction if keypad[VX] is not pressed
                0xA1 => Ok(Instruction::InsEXA1(x)),
                _ => Err(InstructionError::UnknownOpcode(opcode)),
            },
            0xF000 => match nn {
                // FX07: VX = delay_timer
                0x07 => Ok(Instruction::InsFX07(x)),
                // FX0A: Wait for key press, store key in VX
                0x0A => Ok(Instruction::InsFX0A(x)),
                // FX15: delay_timer = VX
                0x15 => Ok(Instruction::InsFX15(x)),
                // FX18: sound_timer = VX
                0x18 => Ok(Instruction::InsFX18(x)),
                // FX1E: I += VX
                0x1E => Ok(Instruction::InsFX1E(x)),
                // FX29: I = address of font in VX
                0x29 => Ok(Instruction::InsFX29(x)),
                // FX33: Binary-coded decimal representation of VX
                0x33 => Ok(Instruction::InsFX33(x)),
                // FX55: Store V0..VX in memory starting at I
                0x55 => Ok(Instruction::InsFX55(x)),
                // FX65: Load V0..VX from memory starting at I
                0x65 => Ok(Instruction::InsFX65(x)),
                _ => Err(InstructionError::UnknownOpcode(opcode)),
            },
            _ => Err(InstructionError::UnknownOpcode(opcode)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Instruction;
    use crate::error::InstructionError;

    #[test]
    fn decodes_known_valid_opcodes() {
        assert_eq!(Instruction::try_from(0x00E0), Ok(Instruction::Ins00E0));
        assert_eq!(Instruction::try_from(0x00EE), Ok(Instruction::Ins00EE));
        assert_eq!(
            Instruction::try_from(0x5120),
            Ok(Instruction::Ins5XY0(0x1, 0x2))
        );
        assert_eq!(
            Instruction::try_from(0x9230),
            Ok(Instruction::Ins9XY0(0x2, 0x3))
        );
        assert_eq!(Instruction::try_from(0xE49E), Ok(Instruction::InsEX9E(0x4)));
        assert_eq!(Instruction::try_from(0xEAA1), Ok(Instruction::InsEXA1(0xA)));
        assert_eq!(Instruction::try_from(0xF229), Ok(Instruction::InsFX29(0x2)));
        assert_eq!(Instruction::try_from(0xF333), Ok(Instruction::InsFX33(0x3)));
        assert_eq!(Instruction::try_from(0xFA55), Ok(Instruction::InsFX55(0xA)));
        assert_eq!(Instruction::try_from(0xFB65), Ok(Instruction::InsFX65(0xB)));
    }

    #[test]
    fn rejects_opcode_prefixes_that_only_partially_match() {
        for opcode in [
            0x0010, 0x00FE, 0x5121, 0x9234, 0xE490, 0xEAAE, 0xF22A, 0xF334, 0xFA5A, 0xFB6A,
        ] {
            assert_eq!(
                Instruction::try_from(opcode),
                Err(InstructionError::UnknownOpcode(opcode))
            );
        }
    }
}
