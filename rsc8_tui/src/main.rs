use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::{event, execute, style::Print},
    layout::Rect,
    style::{Color, Style},
    symbols::shade,
    widgets::Widget,
};
use rsc8_core::{
    chip8::{Chip8, SCREEN_TRAIL_FADE_FRAMES},
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
/// Number of terminal cells the screen is wide in either resolution.
const SCREEN_CELLS_WIDTH: u16 = 128;
const PIXEL_STYLE: Style = Style::new().fg(Color::White);
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
    ratatui::run(run)
}

fn run(terminal: &mut DefaultTerminal) -> Result<(), Box<dyn Error>> {
    let rng = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(unix_timestamp) => LinearCongruentialGenerator {
            seed: unix_timestamp.as_millis() as u16,
        },
        Err(_) => LinearCongruentialGenerator::default(),
    };

    let mut chip8 = Chip8::new(rng);
    chip8.load_fontset();

    {
        let options = Options::from_args(args().skip(1))?;
        let mut f = File::open(options.rom)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        chip8.load_rom(&buffer)?;
        chip8.shift_in_place = !options.vip_quirks;
    }

    let frame_time = Duration::from_millis(1000 / FRAME_RATE);
    let mut keypad_reset_countdown = KEYPAD_RESET_COUNTDOWN_INIT;

    loop {
        let frame_start = Instant::now();

        chip8.fade_screen();

        for _ in 0..TICK_PER_FRAME {
            if chip8.wait_for_key_release.is_some() {
                break;
            }
            chip8.tick()?;
        }

        chip8.tick_timer();

        if chip8.sound_timer > 0 {
            execute!(std::io::stdout(), Print('\x07'))?;
        }

        terminal.draw(|frame| {
            let (width, height) = chip8.screen_dimensions();
            frame.render_widget(
                Screen {
                    trail: chip8.screen_trail_pixels(),
                    width,
                    height,
                },
                frame.area(),
            );
        })?;

        // Handle every queued event and keep waiting until the frame is due, so
        // bursts of input neither lag behind nor speed up the emulation.
        let deadline = frame_start + frame_time;
        let mut received_event = false;
        while event::poll(deadline.saturating_duration_since(Instant::now()))? {
            received_event = true;
            if let event::Event::Key(key_event) = event::read()?
                && apply_key_event(&mut chip8, key_event) == Control::Quit
            {
                return Ok(());
            }
        }

        if received_event {
            keypad_reset_countdown = KEYPAD_RESET_COUNTDOWN_INIT;
        } else if !cfg!(windows) {
            // Terminals without key release events would otherwise keep keys
            // pressed forever.
            keypad_reset_countdown -= 1;
            if keypad_reset_countdown == 0 {
                keypad_reset_countdown = KEYPAD_RESET_COUNTDOWN_INIT;
                chip8.keypad.fill(false);
                chip8.wait_for_key_release = None;
            }
        }
    }
}

struct Screen<'a> {
    trail: &'a [u8],
    width: usize,
    height: usize,
}

impl Widget for Screen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        // The extended mode has twice the pixels, so it uses half as many cells.
        let pixel_width = (SCREEN_CELLS_WIDTH / self.width as u16).max(1);
        let width = (area.width / pixel_width).min(self.width as u16);
        let height = area.height.min(self.height as u16);
        for y in 0..height {
            for x in 0..width {
                let index = y as usize * self.width + x as usize;
                let Some((glyph, style)) = trail_style(self.trail[index]) else {
                    continue;
                };
                let cell_x = area.x + x * pixel_width;
                for offset in 0..pixel_width {
                    if let Some(cell) = buf.cell_mut((cell_x + offset, area.y + y)) {
                        cell.set_symbol(glyph).set_style(style);
                    }
                }
            }
        }
    }
}

fn trail_style(trail: u8) -> Option<(&'static str, Style)> {
    const FADING_SHADES: [&str; SCREEN_TRAIL_FADE_FRAMES as usize] =
        [shade::LIGHT, shade::MEDIUM, shade::DARK];

    if trail == 0 {
        None
    } else if trail > SCREEN_TRAIL_FADE_FRAMES {
        Some((shade::FULL, PIXEL_STYLE))
    } else {
        Some((FADING_SHADES[trail as usize - 1], PIXEL_STYLE))
    }
}

/// Command line options.
#[derive(Debug, PartialEq, Eq)]
struct Options {
    rom: String,
    /// Use the COSMAC VIP shift quirk instead of the SUPER-CHIP one.
    vip_quirks: bool,
}

impl Options {
    const USAGE: &str = "Usage: rsc8_tui [--vip] <your_rom.ch8>";

    fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut rom = None;
        let mut vip_quirks = false;
        for arg in args {
            if arg == "--vip" {
                vip_quirks = true;
            } else if arg.starts_with('-') {
                return Err(format!("Unknown option {arg}\n{}", Self::USAGE));
            } else {
                rom = Some(arg);
            }
        }
        Ok(Self {
            rom: rom.ok_or_else(|| Self::USAGE.to_string())?,
            vip_quirks,
        })
    }
}

/// Whether the emulator keeps running after a key event.
#[derive(Debug, PartialEq, Eq)]
enum Control {
    Continue,
    Quit,
}

/// Applies a key event to the keypad, mirroring the terminal's key state onto
/// the CHIP-8 keypad.
fn apply_key_event(
    chip8: &mut Chip8<impl Iterator<Item = u16>>,
    key_event: event::KeyEvent,
) -> Control {
    if key_event.code == event::KeyCode::Esc {
        return Control::Quit;
    }
    let Some(key) = pc_key_code_to_chip8_key_code(&key_event.code) else {
        return Control::Continue;
    };
    match key_event.kind {
        event::KeyEventKind::Press | event::KeyEventKind::Repeat => chip8.keypad[key] = true,
        event::KeyEventKind::Release => {
            chip8.keypad[key] = false;
            if chip8.wait_for_key_release == Some(key) {
                chip8.wait_for_key_release = None;
            }
        }
    }
    Control::Continue
}

fn pc_key_code_to_chip8_key_code(key_code: &event::KeyCode) -> Option<usize> {
    if let event::KeyCode::Char(c) = key_code {
        let c = c.to_ascii_lowercase();
        KEY_MAP
            .iter()
            .find(|(key, _)| key == &c)
            .map(|(_, code)| *code)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsc8_core::chip8::{
        LOW_RES_SCREEN_HEIGHT, LOW_RES_SCREEN_WIDTH, SCREEN_HEIGHT, SCREEN_TRAIL_FULL, SCREEN_WIDTH,
    };

    fn screen(trail: &[u8], width: usize, height: usize) -> Screen<'_> {
        Screen {
            trail,
            width,
            height,
        }
    }

    #[test]
    fn vip_quirks_are_opt_in() {
        let options = |args: &[&str]| Options::from_args(args.iter().map(|arg| arg.to_string()));

        let default = options(&["game.ch8"]).unwrap();
        assert_eq!(default.rom, "game.ch8");
        assert!(!default.vip_quirks);

        let vip = options(&["--vip", "game.ch8"]).unwrap();
        assert_eq!(vip.rom, "game.ch8");
        assert!(vip.vip_quirks);

        assert_eq!(
            options(&["--vip"]).unwrap_err(),
            "Usage: rsc8_tui [--vip] <your_rom.ch8>"
        );
        assert!(
            options(&["--nope", "game.ch8"])
                .unwrap_err()
                .contains("Unknown option --nope")
        );
    }

    #[test]
    fn key_events_mirror_the_keypad() {
        let mut chip8 = Chip8::new(core::iter::repeat(0_u16));
        let press = |kind| {
            event::KeyEvent::new_with_kind(
                event::KeyCode::Char('q'),
                event::KeyModifiers::NONE,
                kind,
            )
        };

        assert_eq!(
            apply_key_event(&mut chip8, press(event::KeyEventKind::Press)),
            Control::Continue
        );
        assert!(chip8.keypad[0x4]);

        assert_eq!(
            apply_key_event(&mut chip8, press(event::KeyEventKind::Repeat)),
            Control::Continue
        );
        assert!(chip8.keypad[0x4]);

        chip8.wait_for_key_release = Some(0x4);
        assert_eq!(
            apply_key_event(&mut chip8, press(event::KeyEventKind::Release)),
            Control::Continue
        );
        assert!(!chip8.keypad[0x4]);
        assert_eq!(chip8.wait_for_key_release, None);

        // Releasing an unrelated key leaves the wait alone.
        chip8.keypad[0x1] = true;
        chip8.wait_for_key_release = Some(0x1);
        let release_other = event::KeyEvent::new_with_kind(
            event::KeyCode::Char('v'),
            event::KeyModifiers::NONE,
            event::KeyEventKind::Release,
        );
        apply_key_event(&mut chip8, release_other);
        assert_eq!(chip8.wait_for_key_release, Some(0x1));

        let escape = event::KeyEvent::new_with_kind(
            event::KeyCode::Esc,
            event::KeyModifiers::NONE,
            event::KeyEventKind::Press,
        );
        assert_eq!(apply_key_event(&mut chip8, escape), Control::Quit);

        let unmapped = event::KeyEvent::new_with_kind(
            event::KeyCode::Char(' '),
            event::KeyModifiers::NONE,
            event::KeyEventKind::Press,
        );
        assert_eq!(apply_key_event(&mut chip8, unmapped), Control::Continue);
        assert_eq!(chip8.keypad.iter().filter(|pressed| **pressed).count(), 1);
    }

    #[test]
    fn key_mapping_ignores_the_case_of_letter_keys() {
        let key = |code| pc_key_code_to_chip8_key_code(&event::KeyCode::Char(code));
        assert_eq!(key('q'), Some(0x4));
        assert_eq!(key('Q'), Some(0x4));
        assert_eq!(key('v'), Some(0xF));
        assert_eq!(key('V'), Some(0xF));
        assert_eq!(key('1'), Some(0x1));
        assert_eq!(key(' '), None);
        assert_eq!(pc_key_code_to_chip8_key_code(&event::KeyCode::Esc), None);
    }

    #[test]
    fn trail_style_fades_visible_pixels_into_lighter_shades() {
        assert_eq!(trail_style(0), None);

        let glyph = |trail: u8| trail_style(trail).map(|(glyph, _)| glyph);
        for trail in 1..=SCREEN_TRAIL_FADE_FRAMES {
            assert!(glyph(trail).is_some(), "trail {trail} must stay visible");
        }

        for trail in SCREEN_TRAIL_FADE_FRAMES + 1..=SCREEN_TRAIL_FULL {
            assert_eq!(glyph(trail), Some(shade::FULL));
        }
        let faded: Vec<&str> = (1..=SCREEN_TRAIL_FADE_FRAMES).filter_map(glyph).collect();
        assert_eq!(faded.first().copied(), Some(shade::LIGHT));
        assert_eq!(faded.last().copied(), Some(shade::DARK));
    }

    #[test]
    fn screen_draws_low_res_pixels_two_cells_wide() {
        let mut trail = [0_u8; LOW_RES_SCREEN_WIDTH * LOW_RES_SCREEN_HEIGHT];
        trail[0] = SCREEN_TRAIL_FULL;
        trail[1] = 1;

        let area = Rect::new(0, 0, 8, 1);
        let mut buffer = Buffer::empty(area);
        screen(&trail, LOW_RES_SCREEN_WIDTH, LOW_RES_SCREEN_HEIGHT).render(area, &mut buffer);

        let row: Vec<&str> = (0..8_u16).map(|x| buffer[(x, 0)].symbol()).collect();
        assert_eq!(
            row,
            [
                shade::FULL,
                shade::FULL,
                shade::LIGHT,
                shade::LIGHT,
                " ",
                " ",
                " ",
                " "
            ]
        );
    }

    #[test]
    fn screen_draws_hi_res_pixels_one_cell_wide() {
        let mut trail = [0_u8; SCREEN_WIDTH * SCREEN_HEIGHT];
        trail[0] = SCREEN_TRAIL_FULL;
        trail[1] = SCREEN_TRAIL_FULL;

        let area = Rect::new(0, 0, 4, 1);
        let mut buffer = Buffer::empty(area);
        screen(&trail, SCREEN_WIDTH, SCREEN_HEIGHT).render(area, &mut buffer);

        let row: Vec<&str> = (0..4_u16).map(|x| buffer[(x, 0)].symbol()).collect();
        assert_eq!(row, [shade::FULL, shade::FULL, " ", " "]);
    }

    #[test]
    fn screen_renders_at_the_offset_of_its_area() {
        let mut trail = [0_u8; LOW_RES_SCREEN_WIDTH * LOW_RES_SCREEN_HEIGHT];
        trail[0] = SCREEN_TRAIL_FULL;

        let buffer_area = Rect::new(0, 0, 8, 4);
        let mut buffer = Buffer::empty(buffer_area);
        screen(&trail, LOW_RES_SCREEN_WIDTH, LOW_RES_SCREEN_HEIGHT)
            .render(Rect::new(2, 1, 4, 2), &mut buffer);

        assert_eq!(buffer[(2, 1)].symbol(), shade::FULL);
        assert_eq!(buffer[(3, 1)].symbol(), shade::FULL);
        assert_eq!(buffer[(0, 0)].symbol(), " ");
    }

    #[test]
    fn screen_clips_to_small_areas_and_buffers() {
        let mut trail = [0_u8; LOW_RES_SCREEN_WIDTH * LOW_RES_SCREEN_HEIGHT];
        trail[(LOW_RES_SCREEN_HEIGHT - 1) * LOW_RES_SCREEN_WIDTH] = SCREEN_TRAIL_FULL;

        let area = Rect::new(0, 0, 3, 1);
        let mut buffer = Buffer::empty(area);
        screen(&trail, LOW_RES_SCREEN_WIDTH, LOW_RES_SCREEN_HEIGHT).render(
            Rect::new(0, 0, SCREEN_CELLS_WIDTH, SCREEN_HEIGHT as u16),
            &mut buffer,
        );

        assert_eq!(buffer[(0, 0)].symbol(), " ");
    }
}
