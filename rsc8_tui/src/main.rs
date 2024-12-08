use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    layout::Rect,
    style::{Style, Stylize},
    widgets::Block,
    DefaultTerminal,
};
use rsc8_core::chip8;
use std::{
    env::args,
    error,
    fs::File,
    io::Read,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FRAME_RATE: u64 = 60;
const KEYPAD_RESET_COUNTDOWN_INIT: u64 = 10;
const TICK_PER_FRAME: u8 = 8;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<(), Box<dyn error::Error>> {
    // Init chip8
    let mut chip8 = chip8::Chip8::default();

    // Try set random seed
    if let Ok(unix_timestamp) = SystemTime::now().duration_since(UNIX_EPOCH) {
        chip8.rng.seed = unix_timestamp.as_millis() as u16;
    }

    // Load fontset
    chip8.load_fontset();

    // Load rom
    {
        let filename = args().nth(1).ok_or("Usage: rsc8_tui <your_rom.ch8>")?;
        let mut f = File::open(filename)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        chip8.load_rom(&buffer);
    }

    let tick_rate = Duration::from_millis(1000 / FRAME_RATE);
    let mut last_tick = Instant::now();

    let mut keypad_reset_countdown = KEYPAD_RESET_COUNTDOWN_INIT;

    loop {
        // Tick
        for _ in 0..TICK_PER_FRAME {
            if chip8.wait_for_key_release.0 {
                break;
            }
            chip8.tick()?;
        }

        // Tick timer
        chip8.tick_timer();

        // Beep
        if chip8.sound_timer > 0 {
            print!("\x07");
        }

        // Draw screen
        if chip8.draw_flag {
            chip8.draw_flag = false;
            terminal.draw(|frame| {
                chip8.screen.iter().enumerate().for_each(|(index, pixel)| {
                    if *pixel {
                        let x = (index % chip8::SCREEN_WIDTH) as u16;
                        let y = (index / chip8::SCREEN_WIDTH) as u16;
                        let area = Rect::new(x * 2, y, 2, 1);
                        let block = Block::default().style(Style::new().on_white());
                        frame.render_widget(block, area);
                    }
                });
            })?;
        }

        // Update keypad
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            keypad_reset_countdown = KEYPAD_RESET_COUNTDOWN_INIT;
            if let event::Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Esc && key_event.kind == KeyEventKind::Press {
                    return Ok(());
                }
                if let Some(chip8_key_code) = pc_key_code_to_chip8_key_code(&key_event.code) {
                    match key_event.kind {
                        KeyEventKind::Press => chip8.keypad[chip8_key_code] = true,
                        KeyEventKind::Release => {
                            chip8.keypad[chip8_key_code] = false;
                            if chip8_key_code == chip8.wait_for_key_release.1 {
                                chip8.wait_for_key_release.0 = false;
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else if !cfg!(windows) {
            keypad_reset_countdown -= 1;
            if keypad_reset_countdown == 0 {
                keypad_reset_countdown = KEYPAD_RESET_COUNTDOWN_INIT;
                chip8.keypad.iter_mut().for_each(|pressed| *pressed = false);
                if chip8.wait_for_key_release.0 {
                    chip8.wait_for_key_release.0 = false;
                }
            }
        }

        // Update last tick
        last_tick = Instant::now();
    }
}

fn pc_key_code_to_chip8_key_code(key_code: &KeyCode) -> Option<usize> {
    match key_code {
        KeyCode::Char('1') => Some(0x1),
        KeyCode::Char('2') => Some(0x2),
        KeyCode::Char('3') => Some(0x3),
        KeyCode::Char('4') => Some(0xC),
        KeyCode::Char('q') => Some(0x4),
        KeyCode::Char('w') => Some(0x5),
        KeyCode::Char('e') => Some(0x6),
        KeyCode::Char('r') => Some(0xD),
        KeyCode::Char('a') => Some(0x7),
        KeyCode::Char('s') => Some(0x8),
        KeyCode::Char('d') => Some(0x9),
        KeyCode::Char('f') => Some(0xE),
        KeyCode::Char('z') => Some(0xA),
        KeyCode::Char('x') => Some(0x0),
        KeyCode::Char('c') => Some(0xB),
        KeyCode::Char('v') => Some(0xF),
        _ => None,
    }
}
