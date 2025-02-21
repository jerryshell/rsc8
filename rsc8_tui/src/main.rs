use ratatui::{
    DefaultTerminal,
    crossterm::event,
    layout::Rect,
    style::{Style, Stylize},
    widgets::Block,
};
use rsc8_core::{
    chip8::{Chip8, SCREEN_WIDTH},
    rng::LinearCongruentialGenerator,
};
use std::{
    env::args,
    error::Error,
    fs::File,
    io::Read,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FRAME_RATE: u64 = 60;
const KEYPAD_RESET_COUNTDOWN_INIT: u64 = 10;
const TICK_PER_FRAME: u8 = 8;
const KEY_MAP: [(char, usize); 16] = [
    ('1', 0x1),
    ('2', 0x2),
    ('3', 0x3),
    ('4', 0xC),
    ('q', 0x4),
    ('w', 0x5),
    ('e', 0x6),
    ('r', 0xD),
    ('a', 0x7),
    ('s', 0x8),
    ('d', 0x9),
    ('f', 0xE),
    ('z', 0xA),
    ('x', 0x0),
    ('c', 0xB),
    ('v', 0xF),
];

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
    // Init rng
    let rng = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(unix_timestamp) => LinearCongruentialGenerator {
            seed: unix_timestamp.as_millis() as u16,
        },
        Err(_) => LinearCongruentialGenerator::default(),
    };

    // Init chip8
    let mut chip8 = Chip8::new(rng);

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
            if chip8.wait_for_key_release.is_some() {
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
                        let x = (index % SCREEN_WIDTH) as u16;
                        let y = (index / SCREEN_WIDTH) as u16;
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
                if key_event.code == event::KeyCode::Esc {
                    return Ok(());
                }
                if let Some(chip8_key_code) = pc_key_code_to_chip8_key_code(&key_event.code) {
                    match key_event.kind {
                        event::KeyEventKind::Press => chip8.keypad[chip8_key_code] = true,
                        event::KeyEventKind::Release => {
                            chip8.keypad[chip8_key_code] = false;
                            if let Some(key_code) = chip8.wait_for_key_release {
                                if chip8_key_code == key_code {
                                    chip8.wait_for_key_release = None;
                                }
                            }
                        }
                        event::KeyEventKind::Repeat => {}
                    }
                }
            }
        } else if !cfg!(windows) {
            keypad_reset_countdown -= 1;
            if keypad_reset_countdown == 0 {
                keypad_reset_countdown = KEYPAD_RESET_COUNTDOWN_INIT;
                chip8.keypad.iter_mut().for_each(|pressed| *pressed = false);
                chip8.wait_for_key_release = None;
            }
        }

        // Update last tick
        last_tick = Instant::now();
    }
}

fn pc_key_code_to_chip8_key_code(key_code: &event::KeyCode) -> Option<usize> {
    if let event::KeyCode::Char(c) = key_code {
        KEY_MAP
            .iter()
            .find(|(key, _)| key == c)
            .map(|(_, code)| *code)
    } else {
        None
    }
}
