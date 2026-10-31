use crate::{error::InstructionError, instruction::Instruction};

pub const MEMORY_SIZE: usize = 4096;
pub const NUM_REGISTERS: usize = 16;
pub const STACK_SIZE: usize = 16;
pub const KEYPAD_SIZE: usize = 16;
/// Width of the extended (SCHIP) screen, which is also the size of the screen
/// buffers.
pub const SCREEN_WIDTH: usize = 128;
/// Height of the extended (SCHIP) screen.
pub const SCREEN_HEIGHT: usize = 64;
/// Width of the screen in the default (CHIP-8) mode.
pub const LOW_RES_SCREEN_WIDTH: usize = 64;
/// Height of the screen in the default (CHIP-8) mode.
pub const LOW_RES_SCREEN_HEIGHT: usize = 32;
/// Number of frames a pixel stays fully lit after it stops being drawn, so that
/// sprites erased and redrawn a frame or two later do not blink.
pub const SCREEN_TRAIL_HOLD_FRAMES: u8 = 2;
/// Number of frames a pixel then fades out over.
pub const SCREEN_TRAIL_FADE_FRAMES: u8 = 3;
/// Trail level of a pixel that was just drawn.
pub const SCREEN_TRAIL_FULL: u8 = SCREEN_TRAIL_HOLD_FRAMES + SCREEN_TRAIL_FADE_FRAMES;
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
    /// Whether a pixel is currently lit. Rows are `screen_width()` pixels wide
    /// and only the first `screen_width() * screen_height()` entries are used.
    pub screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    /// Display brightness of a pixel, from 0 (invisible) to `SCREEN_TRAIL_FULL`.
    /// Values above `SCREEN_TRAIL_FADE_FRAMES` are fully lit, the rest fade out.
    /// Erasing a pixel refreshes its trail instead of clearing it.
    pub screen_trail: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    /// Whether the extended (128x64) screen mode is enabled.
    pub hi_res: bool,
    /// Whether `8XY6`/`8XYE` shift `VX` in place instead of shifting `VY` into
    /// `VX`. Games of the SUPER-CHIP era, like David Winter's Space Invaders,
    /// expect this.
    pub shift_in_place: bool,
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
            screen_trail: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            hi_res: false,
            shift_in_place: true,
            draw_flag: false,
            rng,
            wait_for_key_release: None,
        }
    }

    /// Width of the screen in pixels, depending on the screen mode.
    pub fn screen_width(&self) -> usize {
        if self.hi_res {
            SCREEN_WIDTH
        } else {
            LOW_RES_SCREEN_WIDTH
        }
    }

    /// Height of the screen in pixels, depending on the screen mode.
    pub fn screen_height(&self) -> usize {
        if self.hi_res {
            SCREEN_HEIGHT
        } else {
            LOW_RES_SCREEN_HEIGHT
        }
    }

    /// Width and height of the screen in pixels.
    pub fn screen_dimensions(&self) -> (usize, usize) {
        (self.screen_width(), self.screen_height())
    }

    /// The lit pixels of the active screen.
    pub fn screen_pixels(&self) -> &[bool] {
        let (width, height) = self.screen_dimensions();
        &self.screen[..width * height]
    }

    /// The display trail of the active screen, laid out like [`Self::screen`].
    pub fn screen_trail_pixels(&self) -> &[u8] {
        let (width, height) = self.screen_dimensions();
        &self.screen_trail[..width * height]
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

    /// Fades the afterglow of every pixel that is not lit by one step.
    ///
    /// Frontends should call this once per frame, before executing that frame's
    /// instructions.
    pub fn fade_screen(&mut self) {
        let (width, height) = self.screen_dimensions();
        let screen_len = width * height;
        for (pixel, trail) in self.screen[..screen_len]
            .iter()
            .zip(self.screen_trail[..screen_len].iter_mut())
        {
            if !pixel {
                *trail = trail.saturating_sub(1);
            }
        }
    }

    fn set_hi_res(&mut self, hi_res: bool) {
        self.hi_res = hi_res;
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.screen_trail = [0; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.draw_flag = true;
    }

    fn scroll_vertical(&mut self, lines: usize, down: bool) {
        let (width, height) = self.screen_dimensions();
        let screen_len = width * height;
        scroll_vertical(&mut self.screen[..screen_len], width, height, lines, down);
        scroll_vertical(
            &mut self.screen_trail[..screen_len],
            width,
            height,
            lines,
            down,
        );
    }

    fn scroll_sideways(&mut self, pixels: usize, right: bool) {
        let (width, height) = self.screen_dimensions();
        let screen_len = width * height;
        scroll_sideways(&mut self.screen[..screen_len], width, height, pixels, right);
        scroll_sideways(
            &mut self.screen_trail[..screen_len],
            width,
            height,
            pixels,
            right,
        );
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
            Instruction::Ins00CN(n) => {
                self.scroll_vertical(n as usize, true);
                self.draw_flag = true;
            }
            Instruction::Ins00DN(n) => {
                self.scroll_vertical(n as usize, false);
                self.draw_flag = true;
            }
            Instruction::Ins00FB => {
                self.scroll_sideways(4, true);
                self.draw_flag = true;
            }
            Instruction::Ins00FC => {
                self.scroll_sideways(4, false);
                self.draw_flag = true;
            }
            Instruction::Ins00FE => self.set_hi_res(false),
            Instruction::Ins00FF => self.set_hi_res(true),
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
                if !self.shift_in_place {
                    self.register_v[x as usize] = self.register_v[y as usize];
                }
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
                if !self.shift_in_place {
                    self.register_v[x as usize] = self.register_v[y as usize];
                }
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
                let (width, height) = self.screen_dimensions();
                let vx = self.register_v[x as usize] as usize % width;
                let vy = self.register_v[y as usize] as usize % height;
                // A height of zero selects a 16x16 sprite in extended mode.
                let (sprite_width, rows) = if n == 0 { (16, 16) } else { (8, n as usize) };
                self.register_v[0xF] = 0;
                for row in 0..rows {
                    let screen_y = vy + row;
                    if screen_y >= height {
                        break;
                    }
                    let sprite_address = self.register_i as usize + row;
                    let sprite_bits = if sprite_width == 16 {
                        (self.read_memory(sprite_address)? as u32) << 8
                            | self.read_memory(sprite_address + 1)? as u32
                    } else {
                        self.read_memory(sprite_address)? as u32
                    };
                    for col in 0..sprite_width {
                        let screen_x = vx + col;
                        if screen_x >= width {
                            break;
                        }
                        let sprite_pixel = sprite_bits & (1 << (sprite_width - 1 - col)) != 0;
                        let screen_pixel_index = screen_y * width + screen_x;
                        let screen_pixel = self.screen[screen_pixel_index];
                        if sprite_pixel && screen_pixel {
                            self.register_v[0xF] = 1;
                        }
                        self.screen[screen_pixel_index] ^= sprite_pixel;
                        if sprite_pixel {
                            self.screen_trail[screen_pixel_index] = SCREEN_TRAIL_FULL;
                        }
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
                // Only the low nibble names a character, as on the original
                // hardware. Anything larger used to overflow the u8 multiply.
                self.register_i = (self.register_v[x as usize] as u16 & 0xF) * 5;
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

fn scroll_vertical<T: Copy + Default>(
    buffer: &mut [T],
    width: usize,
    height: usize,
    lines: usize,
    down: bool,
) {
    let lines = lines.min(height);
    let screen_len = width * height;
    let shift = width * lines;
    if down {
        buffer.copy_within(0..screen_len - shift, shift);
        buffer[..shift].fill(T::default());
    } else {
        buffer.copy_within(shift..screen_len, 0);
        buffer[screen_len - shift..].fill(T::default());
    }
}

fn scroll_sideways<T: Copy + Default>(
    buffer: &mut [T],
    width: usize,
    height: usize,
    pixels: usize,
    right: bool,
) {
    let pixels = pixels.min(width);
    for y in 0..height {
        let row = &mut buffer[y * width..(y + 1) * width];
        if right {
            row.copy_within(0..width - pixels, pixels);
            row[..pixels].fill(T::default());
        } else {
            row.copy_within(pixels..width, 0);
            row[width - pixels..].fill(T::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_chip8() -> Chip8<core::iter::Repeat<u16>> {
        Chip8::new(core::iter::repeat(0))
    }

    /// Draws the fontset glyph `0` (five rows starting at address 0) at `(x, y)`.
    fn draw_glyph_zero(chip8: &mut Chip8<core::iter::Repeat<u16>>, x: u8, y: u8) {
        chip8.register_v[0] = x;
        chip8.register_v[1] = y;
        chip8.register_i = 0;
        chip8
            .execute_instruction(&Instruction::InsDXYN(0, 1, 5))
            .unwrap();
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
    fn execute_fx29_uses_only_the_low_nibble_and_never_overflows() {
        let mut chip8 = new_chip8();
        chip8.register_v[3] = 0xA;
        assert_eq!(chip8.execute_instruction(&Instruction::InsFX29(3)), Ok(()));
        assert_eq!(chip8.register_i, 0xA * 5);
        // 0x7A is 122, and 122 * 5 does not fit a u8. Its low nibble is 0xA.
        chip8.register_v[3] = 0x7A;
        assert_eq!(chip8.execute_instruction(&Instruction::InsFX29(3)), Ok(()));
        assert_eq!(chip8.register_i, 0xA * 5);
        chip8.register_v[3] = 0xFF;
        assert_eq!(chip8.execute_instruction(&Instruction::InsFX29(3)), Ok(()));
        assert_eq!(chip8.register_i, 0xF * 5);
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

    #[test]
    fn shifts_keep_vx_by_default() {
        let mut chip8 = new_chip8();
        chip8.register_v[1] = 0x0F;
        chip8.register_v[2] = 0xF0;

        chip8
            .execute_instruction(&Instruction::Ins8XY6(1, 2))
            .unwrap();
        assert_eq!(chip8.register_v[1], 0x07);
        assert_eq!(chip8.register_v[2], 0xF0);
        assert_eq!(chip8.register_v[0xF], 1);

        chip8.register_v[1] = 0x0F;
        chip8.register_v[2] = 0xF0;
        chip8
            .execute_instruction(&Instruction::Ins8XYE(1, 2))
            .unwrap();
        assert_eq!(chip8.register_v[1], 0x1E);
        assert_eq!(chip8.register_v[2], 0xF0);
        assert_eq!(chip8.register_v[0xF], 0);
    }

    #[test]
    fn shifts_load_vy_when_the_vip_quirk_is_enabled() {
        let mut chip8 = new_chip8();
        chip8.shift_in_place = false;
        chip8.register_v[1] = 0x0F;
        chip8.register_v[2] = 0xF0;

        chip8
            .execute_instruction(&Instruction::Ins8XY6(1, 2))
            .unwrap();
        assert_eq!(chip8.register_v[1], 0x78);
        assert_eq!(chip8.register_v[2], 0xF0);
        assert_eq!(chip8.register_v[0xF], 0);

        chip8.register_v[1] = 0x0F;
        chip8.register_v[2] = 0xF1;
        chip8
            .execute_instruction(&Instruction::Ins8XYE(1, 2))
            .unwrap();
        assert_eq!(chip8.register_v[1], 0xE2);
        assert_eq!(chip8.register_v[2], 0xF1);
        assert_eq!(chip8.register_v[0xF], 1);
    }

    #[test]
    fn executing_dxyn_lights_pixels_and_refreshes_their_trail() {
        let mut chip8 = new_chip8();
        chip8.load_fontset();
        draw_glyph_zero(&mut chip8, 0, 0);

        // The first row of the glyph is 0xF0, so the left four pixels are lit.
        for index in 0..4 {
            assert!(chip8.screen[index]);
            assert_eq!(chip8.screen_trail[index], SCREEN_TRAIL_FULL);
        }
        assert!(!chip8.screen[4]);
        assert_eq!(chip8.screen_trail[4], 0);
    }

    #[test]
    fn executing_dxyn_erasing_a_pixel_keeps_its_trail_to_fade() {
        let mut chip8 = new_chip8();
        chip8.load_fontset();
        draw_glyph_zero(&mut chip8, 0, 0);
        assert_eq!(chip8.register_v[0xF], 0);

        draw_glyph_zero(&mut chip8, 0, 0);
        assert_eq!(chip8.register_v[0xF], 1);
        assert!(!chip8.screen[0]);
        assert_eq!(chip8.screen_trail[0], SCREEN_TRAIL_FULL);
    }

    #[test]
    fn an_erased_pixel_keeps_full_brightness_during_the_hold() {
        let mut chip8 = new_chip8();
        chip8.load_fontset();
        draw_glyph_zero(&mut chip8, 0, 0);
        draw_glyph_zero(&mut chip8, 0, 0);
        assert!(chip8.screen_trail[0] > SCREEN_TRAIL_FADE_FRAMES);
        for _ in 1..SCREEN_TRAIL_HOLD_FRAMES {
            chip8.fade_screen();
            assert!(chip8.screen_trail[0] > SCREEN_TRAIL_FADE_FRAMES);
        }

        chip8.fade_screen();
        assert!(chip8.screen_trail[0] <= SCREEN_TRAIL_FADE_FRAMES);
    }

    #[test]
    fn collision_ignores_the_fading_trail() {
        let mut chip8 = new_chip8();
        chip8.load_fontset();
        draw_glyph_zero(&mut chip8, 0, 0);
        draw_glyph_zero(&mut chip8, 0, 0);
        assert_eq!(chip8.screen_trail[0], SCREEN_TRAIL_FULL);

        draw_glyph_zero(&mut chip8, 0, 0);
        assert_eq!(chip8.register_v[0xF], 0);
    }

    #[test]
    fn fade_screen_only_decays_pixels_that_are_not_lit() {
        let mut chip8 = new_chip8();
        chip8.screen[0] = true;
        chip8.screen_trail[0] = SCREEN_TRAIL_FULL;
        chip8.screen_trail[1] = 2;

        chip8.fade_screen();
        assert_eq!(chip8.screen_trail[0], SCREEN_TRAIL_FULL);
        assert_eq!(chip8.screen_trail[1], 1);

        chip8.fade_screen();
        assert_eq!(chip8.screen_trail[1], 0);

        chip8.fade_screen();
        assert_eq!(chip8.screen_trail[1], 0);
    }

    #[test]
    fn executing_00e0_clears_the_screen_but_lets_pixels_fade() {
        let mut chip8 = new_chip8();
        chip8.load_fontset();
        draw_glyph_zero(&mut chip8, 0, 0);

        chip8.execute_instruction(&Instruction::Ins00E0).unwrap();
        assert!(chip8.screen.iter().all(|pixel| !pixel));
        assert_eq!(chip8.screen_trail[0], SCREEN_TRAIL_FULL);

        chip8.fade_screen();
        assert_eq!(chip8.screen_trail[0], SCREEN_TRAIL_FULL - 1);
    }

    #[test]
    fn executing_00ff_switches_to_the_extended_screen() {
        let mut chip8 = new_chip8();
        assert_eq!(chip8.screen_dimensions(), (64, 32));
        draw_glyph_zero(&mut chip8, 0, 0);

        chip8.execute_instruction(&Instruction::Ins00FF).unwrap();
        assert!(chip8.hi_res);
        assert_eq!(chip8.screen_dimensions(), (128, 64));
        assert!(chip8.screen.iter().all(|pixel| !pixel));
        assert!(chip8.screen_trail.iter().all(|trail| *trail == 0));

        chip8.execute_instruction(&Instruction::Ins00FE).unwrap();
        assert!(!chip8.hi_res);
        assert_eq!(chip8.screen_dimensions(), (64, 32));
    }

    #[test]
    fn executing_00fb_and_00fc_scroll_the_screen() {
        for hi_res in [false, true] {
            let mut chip8 = new_chip8();
            chip8
                .execute_instruction(&if hi_res {
                    Instruction::Ins00FF
                } else {
                    Instruction::Ins00FE
                })
                .unwrap();
            let (width, _) = chip8.screen_dimensions();
            chip8.screen[width] = true;

            chip8.execute_instruction(&Instruction::Ins00FB).unwrap();
            assert!(!chip8.screen[width]);
            assert!(chip8.screen[width + 4]);

            chip8.execute_instruction(&Instruction::Ins00FC).unwrap();
            assert!(chip8.screen[width]);
            assert!(!chip8.screen[width + 4]);
        }
    }

    #[test]
    fn executing_00cn_and_00dn_scroll_the_screen() {
        for hi_res in [false, true] {
            for down in [true, false] {
                let mut chip8 = new_chip8();
                chip8
                    .execute_instruction(&if hi_res {
                        Instruction::Ins00FF
                    } else {
                        Instruction::Ins00FE
                    })
                    .unwrap();
                let (width, height) = chip8.screen_dimensions();
                let (pixel, moved_off) = if down {
                    (0, (height - 1) * width)
                } else {
                    ((height - 1) * width, 0)
                };
                chip8.screen[pixel] = true;

                let instruction = if down {
                    Instruction::Ins00CN(4)
                } else {
                    Instruction::Ins00DN(4)
                };
                chip8.execute_instruction(&instruction).unwrap();

                assert!(!chip8.screen[pixel]);
                assert!(!chip8.screen[moved_off]);
                let moved_to = if down {
                    pixel + 4 * width
                } else {
                    pixel - 4 * width
                };
                assert!(chip8.screen[moved_to]);
            }
        }
    }

    #[test]
    fn executing_dxyn_with_a_height_of_zero_draws_a_16x16_sprite() {
        let mut chip8 = new_chip8();
        chip8.register_i = 0x300;
        for address in 0x300..0x320 {
            chip8.memory[address] = 0xFF;
        }
        chip8.register_v[0] = 0;
        chip8.register_v[1] = 0;

        chip8
            .execute_instruction(&Instruction::InsDXYN(0, 1, 0))
            .unwrap();

        let width = chip8.screen_width();
        assert!(chip8.screen[0]);
        assert!(chip8.screen[15]);
        assert!(!chip8.screen[16]);
        assert!(chip8.screen[15 * width]);
        assert!(chip8.screen[15 * width + 15]);
        assert!(!chip8.screen[16 * width]);
    }

    #[test]
    fn runs_a_small_extended_mode_rom() {
        let mut chip8 = new_chip8();
        // 00E0, 00FF, A300, 6000, 6100, D010, 00FC
        let rom = [
            0x00, 0xE0, 0x00, 0xFF, 0xA3, 0x00, 0x60, 0x00, 0x61, 0x00, 0xD0, 0x10, 0x00, 0xFC,
        ];
        chip8.load_rom(&rom).unwrap();
        for address in 0x300..0x320 {
            chip8.memory[address] = 0xFF;
        }

        for _ in 0..6 {
            chip8.tick().unwrap();
        }
        assert!(chip8.hi_res);
        assert_eq!(chip8.screen_dimensions(), (128, 64));
        assert!(chip8.screen_pixels()[0]);
        assert!(chip8.screen_pixels()[15]);

        chip8.tick().unwrap();
        assert!(chip8.screen_pixels()[11]);
        assert!(!chip8.screen_pixels()[12]);
        assert!(!chip8.screen_pixels()[127]);
    }
}
