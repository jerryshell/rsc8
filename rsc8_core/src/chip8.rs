use crate::{error::InstructionError, instruction::Instruction};

pub const MEMORY_SIZE: usize = 4096;
pub const NUM_REGISTERS: usize = 16;
pub const STACK_SIZE: usize = 16;
pub const KEYPAD_SIZE: usize = 16;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
pub const PROGRAM_START: u16 = 0x200;
pub const ROM_START: usize = 512;
pub const FONTSET_START: usize = 0;
pub const FONTSET_SIZE: usize = 80;

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Chip8<R>
where
    R: Iterator<Item = u16>,
{
    pub memory: [u8; MEMORY_SIZE],
    pub program_counter: u16,
    pub register_v: [u8; NUM_REGISTERS],
    pub register_i: u16,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub stack: [u16; STACK_SIZE],
    pub stack_pointer: u8,
    pub keypad: [bool; KEYPAD_SIZE],
    pub screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    pub draw_flag: bool,
    pub rng: R,
    pub wait_for_key_release: Option<usize>,
}

impl<R> Chip8<R>
where
    R: Iterator<Item = u16>,
{
    pub fn new(rng: R) -> Self {
        Self {
            memory: [0; MEMORY_SIZE],
            program_counter: PROGRAM_START,
            register_v: [0; NUM_REGISTERS],
            register_i: 0,
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; STACK_SIZE],
            stack_pointer: 0,
            keypad: [false; KEYPAD_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            draw_flag: false,
            rng,
            wait_for_key_release: None,
        }
    }

    pub fn load_fontset(&mut self) {
        self.memory[..FONTSET.len()].copy_from_slice(&FONTSET);
    }

    pub fn load_rom(&mut self, buffer: &[u8]) -> Result<(), InstructionError> {
        let max_size = MEMORY_SIZE - ROM_START;
        if buffer.len() > max_size {
            return Err(InstructionError::RomTooLarge {
                rom_size: buffer.len(),
                max_size,
            });
        }
        let rom_end = ROM_START + buffer.len();
        self.memory[ROM_START..rom_end].copy_from_slice(buffer);
        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), InstructionError> {
        let opcode = self.fetch_opcode()?;
        let instruction = Instruction::try_from(opcode)?;
        self.execute_instruction(&instruction)
    }

    pub fn tick_timer(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    pub fn fetch_opcode(&mut self) -> Result<u16, InstructionError> {
        let pc = self.program_counter as usize;
        if pc + 1 >= MEMORY_SIZE {
            return Err(InstructionError::ProgramCounterOutOfBounds(
                self.program_counter,
            ));
        }
        let high_byte = self.memory[pc];
        let low_byte = self.memory[pc + 1];

        // Increment program counter
        self.program_counter =
            self.program_counter
                .checked_add(2)
                .ok_or(InstructionError::ProgramCounterOverflow(
                    self.program_counter,
                ))?;

        Ok(((high_byte as u16) << 8) | low_byte as u16)
    }

    fn read_memory(&self, address: usize) -> Result<u8, InstructionError> {
        self.memory
            .get(address)
            .copied()
            .ok_or(InstructionError::MemoryOutOfBounds(address))
    }

    fn write_memory(&mut self, address: usize, value: u8) -> Result<(), InstructionError> {
        if let Some(cell) = self.memory.get_mut(address) {
            *cell = value;
            Ok(())
        } else {
            Err(InstructionError::MemoryOutOfBounds(address))
        }
    }

    fn skip_next_instruction(&mut self) -> Result<(), InstructionError> {
        self.program_counter =
            self.program_counter
                .checked_add(2)
                .ok_or(InstructionError::ProgramCounterOverflow(
                    self.program_counter,
                ))?;
        Ok(())
    }

    fn repeat_current_instruction(&mut self) -> Result<(), InstructionError> {
        self.program_counter = self.program_counter.checked_sub(2).ok_or(
            InstructionError::ProgramCounterUnderflow(self.program_counter),
        )?;
        Ok(())
    }

    fn push_stack(&mut self, value: u16) -> Result<(), InstructionError> {
        let stack_index = self.stack_pointer as usize;
        if stack_index >= STACK_SIZE {
            return Err(InstructionError::StackOverflow);
        }
        self.stack[stack_index] = value;
        self.stack_pointer += 1;
        Ok(())
    }

    fn pop_stack(&mut self) -> Result<u16, InstructionError> {
        if self.stack_pointer == 0 {
            return Err(InstructionError::StackUnderflow);
        }
        self.stack_pointer -= 1;
        Ok(self.stack[self.stack_pointer as usize])
    }

    fn keypad_state_for_register(&self, register_index: u8) -> Result<bool, InstructionError> {
        let key_index = self.register_v[register_index as usize];
        if key_index as usize >= KEYPAD_SIZE {
            return Err(InstructionError::InvalidKeyIndex(key_index));
        }
        Ok(self.keypad[key_index as usize])
    }

    pub fn execute_instruction(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(), InstructionError> {
        match *instruction {
            Instruction::Ins00E0 => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
                self.draw_flag = true;
            }
            Instruction::Ins00EE => {
                self.program_counter = self.pop_stack()?;
            }
            Instruction::Ins1NNN(nnn) => {
                self.program_counter = nnn;
            }
            Instruction::Ins2NNN(nnn) => {
                self.push_stack(self.program_counter)?;
                self.program_counter = nnn;
            }
            Instruction::Ins3XNN(x, nn) => {
                if self.register_v[x as usize] == nn {
                    self.skip_next_instruction()?;
                }
            }
            Instruction::Ins4XNN(x, nn) => {
                if self.register_v[x as usize] != nn {
                    self.skip_next_instruction()?;
                }
            }
            Instruction::Ins5XY0(x, y) => {
                if self.register_v[x as usize] == self.register_v[y as usize] {
                    self.skip_next_instruction()?;
                }
            }
            Instruction::Ins6XNN(x, nn) => {
                self.register_v[x as usize] = nn;
            }
            Instruction::Ins7XNN(x, nn) => {
                self.register_v[x as usize] = self.register_v[x as usize].wrapping_add(nn);
            }
            Instruction::Ins8XY0(x, y) => {
                self.register_v[x as usize] = self.register_v[y as usize];
            }
            Instruction::Ins8XY1(x, y) => {
                self.register_v[x as usize] |= self.register_v[y as usize];
                self.register_v[0xF] = 0;
            }
            Instruction::Ins8XY2(x, y) => {
                self.register_v[x as usize] &= self.register_v[y as usize];
                self.register_v[0xF] = 0;
            }
            Instruction::Ins8XY3(x, y) => {
                self.register_v[x as usize] ^= self.register_v[y as usize];
                self.register_v[0xF] = 0;
            }
            Instruction::Ins8XY4(x, y) => {
                let (result, carry) =
                    self.register_v[x as usize].overflowing_add(self.register_v[y as usize]);
                self.register_v[x as usize] = result;
                self.register_v[0xF] = carry as u8;
            }
            Instruction::Ins8XY5(x, y) => {
                let (result, carry) =
                    self.register_v[x as usize].overflowing_sub(self.register_v[y as usize]);
                self.register_v[x as usize] = result;
                self.register_v[0xF] = !carry as u8;
            }
            Instruction::Ins8XY6(x, y) => {
                self.register_v[x as usize] = self.register_v[y as usize];
                let lsb = self.register_v[x as usize] & 1;
                self.register_v[x as usize] >>= 1;
                self.register_v[0xF] = lsb;
            }
            Instruction::Ins8XY7(x, y) => {
                let (result, carry) =
                    self.register_v[y as usize].overflowing_sub(self.register_v[x as usize]);
                self.register_v[x as usize] = result;
                self.register_v[0xF] = !carry as u8;
            }
            Instruction::Ins8XYE(x, y) => {
                self.register_v[x as usize] = self.register_v[y as usize];
                let msb = self.register_v[x as usize] >> 7;
                self.register_v[x as usize] <<= 1;
                self.register_v[0xF] = msb;
            }
            Instruction::Ins9XY0(x, y) => {
                if self.register_v[x as usize] != self.register_v[y as usize] {
                    self.skip_next_instruction()?;
                }
            }
            Instruction::InsANNN(nnn) => {
                self.register_i = nnn;
            }
            Instruction::InsBNNN(nnn) => {
                self.program_counter = nnn + self.register_v[0] as u16;
            }
            Instruction::InsCXNN(x, nn) => {
                let random = self.rng.next().unwrap_or_default();
                self.register_v[x as usize] = random as u8 & nn;
            }
            Instruction::InsDXYN(x, y, n) => {
                let vx = self.register_v[x as usize] % SCREEN_WIDTH as u8;
                let vy = self.register_v[y as usize] % SCREEN_HEIGHT as u8;
                self.register_v[0xF] = 0;
                for row in 0..n {
                    let screen_y = vy + row;
                    if screen_y >= SCREEN_HEIGHT as u8 {
                        break;
                    }
                    let sprite_address = self.register_i as usize + row as usize;
                    let sprite_row = self.read_memory(sprite_address)?;
                    for col in 0..8 {
                        let screen_x = vx + col;
                        if screen_x >= SCREEN_WIDTH as u8 {
                            break;
                        }
                        let sprite_pixel = (sprite_row & (0b1000_0000 >> col)) != 0;
                        let screen_pixel_index =
                            screen_x as usize + screen_y as usize * SCREEN_WIDTH;
                        let screen_pixel = self.screen[screen_pixel_index];
                        if sprite_pixel && screen_pixel {
                            self.register_v[0xF] = 1;
                        }
                        self.screen[screen_pixel_index] ^= sprite_pixel;
                    }
                }
                self.draw_flag = true;
            }
            Instruction::InsEX9E(x) => {
                if self.keypad_state_for_register(x)? {
                    self.skip_next_instruction()?;
                }
            }
            Instruction::InsEXA1(x) => {
                if !self.keypad_state_for_register(x)? {
                    self.skip_next_instruction()?;
                }
            }
            Instruction::InsFX07(x) => {
                self.register_v[x as usize] = self.delay_timer;
            }
            Instruction::InsFX0A(x) => {
                let mut any_key_pressed = false;
                for (key_code, &key_pressed) in self.keypad.iter().enumerate() {
                    if key_pressed {
                        any_key_pressed = true;
                        self.wait_for_key_release = Some(key_code);
                        self.register_v[x as usize] = key_code as u8;
                        break;
                    }
                }
                if !any_key_pressed {
                    self.repeat_current_instruction()?;
                }
            }
            Instruction::InsFX15(x) => {
                self.delay_timer = self.register_v[x as usize];
            }
            Instruction::InsFX18(x) => {
                self.sound_timer = self.register_v[x as usize];
            }
            Instruction::InsFX1E(x) => {
                self.register_i = self
                    .register_i
                    .wrapping_add(self.register_v[x as usize] as u16);
            }
            Instruction::InsFX29(x) => {
                self.register_i = (self.register_v[x as usize] * 5) as u16;
            }
            Instruction::InsFX33(x) => {
                let hundreds = self.register_v[x as usize] / 100;
                let tens = (self.register_v[x as usize] / 10) % 10;
                let ones = self.register_v[x as usize] % 10;
                self.write_memory(self.register_i as usize, hundreds)?;
                self.write_memory(self.register_i as usize + 1, tens)?;
                self.write_memory(self.register_i as usize + 2, ones)?;
            }
            Instruction::InsFX55(x) => {
                for index in 0..=x {
                    let address = self.register_i as usize + index as usize;
                    self.write_memory(address, self.register_v[index as usize])?;
                }
                self.register_i = self.register_i.wrapping_add(x as u16 + 1);
            }
            Instruction::InsFX65(x) => {
                for index in 0..=x {
                    let address = self.register_i as usize + index as usize;
                    self.register_v[index as usize] = self.read_memory(address)?;
                }
                self.register_i = self.register_i.wrapping_add(x as u16 + 1);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_chip8() -> Chip8<core::iter::Repeat<u16>> {
        Chip8::new(core::iter::repeat(0))
    }

    #[test]
    fn load_rom_rejects_too_large_buffer() {
        let mut chip8 = new_chip8();
        let buffer = [0_u8; MEMORY_SIZE - ROM_START + 1];
        assert_eq!(
            chip8.load_rom(&buffer),
            Err(InstructionError::RomTooLarge {
                rom_size: buffer.len(),
                max_size: MEMORY_SIZE - ROM_START,
            })
        );
    }

    #[test]
    fn fetch_opcode_rejects_out_of_bounds_program_counter() {
        let mut chip8 = new_chip8();
        chip8.program_counter = (MEMORY_SIZE - 1) as u16;
        assert_eq!(
            chip8.fetch_opcode(),
            Err(InstructionError::ProgramCounterOutOfBounds(
                chip8.program_counter
            ))
        );
    }

    #[test]
    fn execute_00ee_rejects_stack_underflow() {
        let mut chip8 = new_chip8();
        assert_eq!(
            chip8.execute_instruction(&Instruction::Ins00EE),
            Err(InstructionError::StackUnderflow)
        );
    }

    #[test]
    fn execute_2nnn_rejects_stack_overflow() {
        let mut chip8 = new_chip8();
        chip8.stack_pointer = STACK_SIZE as u8;
        assert_eq!(
            chip8.execute_instruction(&Instruction::Ins2NNN(0x300)),
            Err(InstructionError::StackOverflow)
        );
    }

    #[test]
    fn execute_ex9e_rejects_invalid_key_index() {
        let mut chip8 = new_chip8();
        chip8.register_v[1] = 0xFF;
        assert_eq!(
            chip8.execute_instruction(&Instruction::InsEX9E(1)),
            Err(InstructionError::InvalidKeyIndex(0xFF))
        );
    }

    #[test]
    fn execute_fx33_rejects_out_of_bounds_memory_write() {
        let mut chip8 = new_chip8();
        chip8.register_i = (MEMORY_SIZE - 1) as u16;
        chip8.register_v[0] = 255;
        assert_eq!(
            chip8.execute_instruction(&Instruction::InsFX33(0)),
            Err(InstructionError::MemoryOutOfBounds(MEMORY_SIZE))
        );
    }
}
