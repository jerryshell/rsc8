use crate::{error::InstructionError, instruction::Instruction};

pub const MEMORY_SIZE: usize = 4096;
pub const PROGRAM_START: u16 = 0x200;
pub const NUM_REGISTERS: usize = 16;
pub const FONTSET_START: usize = 0;
pub const FONTSET_SIZE: usize = 80;
pub const STACK_SIZE: usize = 16;
pub const KEYPAD_SIZE: usize = 16;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

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

#[repr(C)]
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
            keypad: [false; 16],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            draw_flag: false,
            rng,
            wait_for_key_release: None,
        }
    }

    pub fn load_fontset(&mut self) {
        self.memory[..FONTSET.len()].copy_from_slice(&FONTSET);
    }

    pub fn load_rom(&mut self, buffer: &[u8]) {
        self.memory[512..(buffer.len() + 512)].copy_from_slice(buffer);
    }

    pub fn tick(&mut self) -> Result<(), InstructionError> {
        let opcode = self.fetch_opcode();
        let instruction = Instruction::try_from(opcode)?;
        self.execute_instruction(&instruction);
        Ok(())
    }

    pub fn tick_timer(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    pub fn fetch_opcode(&mut self) -> u16 {
        let high_byte = self.memory[self.program_counter as usize];
        let low_byte = self.memory[self.program_counter as usize + 1];

        // Increment program counter
        self.program_counter += 2;

        (high_byte as u16) << 8 | low_byte as u16
    }

    pub fn execute_instruction(&mut self, instruction: &Instruction) {
        match *instruction {
            Instruction::Ins00E0 => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
                self.draw_flag = true;
            }
            Instruction::Ins00EE => {
                self.stack_pointer -= 1;
                self.program_counter = self.stack[self.stack_pointer as usize];
            }
            Instruction::Ins1NNN(nnn) => {
                self.program_counter = nnn;
            }
            Instruction::Ins2NNN(nnn) => {
                self.stack[self.stack_pointer as usize] = self.program_counter;
                self.stack_pointer += 1;
                self.program_counter = nnn;
            }
            Instruction::Ins3XNN(x, nn) => {
                if self.register_v[x as usize] == nn {
                    self.program_counter += 2;
                }
            }
            Instruction::Ins4XNN(x, nn) => {
                if self.register_v[x as usize] != nn {
                    self.program_counter += 2;
                }
            }
            Instruction::Ins5XY0(x, y) => {
                if self.register_v[x as usize] == self.register_v[y as usize] {
                    self.program_counter += 2;
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
                    self.program_counter += 2;
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
                    let sprite_row = self.memory[(self.register_i + row as u16) as usize];
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
                if self.keypad[self.register_v[x as usize] as usize] {
                    self.program_counter += 2;
                }
            }
            Instruction::InsEXA1(x) => {
                if !self.keypad[self.register_v[x as usize] as usize] {
                    self.program_counter += 2;
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
                    self.program_counter -= 2;
                }
            }
            Instruction::InsFX15(x) => {
                self.delay_timer = self.register_v[x as usize];
            }
            Instruction::InsFX18(x) => {
                self.sound_timer = self.register_v[x as usize];
            }
            Instruction::InsFX1E(x) => {
                self.register_i += self.register_v[x as usize] as u16;
            }
            Instruction::InsFX29(x) => {
                self.register_i = (self.register_v[x as usize] * 5) as u16;
            }
            Instruction::InsFX33(x) => {
                let hundreds = self.register_v[x as usize] / 100;
                let tens = (self.register_v[x as usize] / 10) % 10;
                let ones = self.register_v[x as usize] % 10;
                self.memory[self.register_i as usize] = hundreds;
                self.memory[(self.register_i + 1) as usize] = tens;
                self.memory[(self.register_i + 2) as usize] = ones;
            }
            Instruction::InsFX55(x) => {
                for index in 0..=x {
                    self.memory[(self.register_i + index as u16) as usize] =
                        self.register_v[index as usize];
                }
                self.register_i += x as u16 + 1;
            }
            Instruction::InsFX65(x) => {
                for index in 0..=x {
                    self.register_v[index as usize] =
                        self.memory[(self.register_i + index as u16) as usize];
                }
                self.register_i += x as u16 + 1;
            }
        }
    }
}
