//! Regression test for the disappearing invaders in David Winter's Space
//! Invaders.
//!
//! The ROM is not vendored. The test looks for it in `RSC8_GAME_ROM_DIR` and in
//! the repository root, and is skipped when it is not there. It needs the
//! default SUPER-CHIP style shifts: with COSMAC VIP style shifts the hit
//! detection shifts the wrong register and invaders vanish in pairs.

use rsc8_core::{
    chip8::{Chip8, KEYPAD_SIZE},
    rng::LinearCongruentialGenerator,
};

use std::{
    fs,
    path::{Path, PathBuf},
};

const ROM: &str = "Space Invaders [David Winter].ch8";
const TICKS_PER_FRAME: usize = 8;
const FIRE_KEY: usize = 5;
const START_FRAME: usize = 60;
const FIRE_INTERVAL: usize = 40;
const FIRE_FRAMES: usize = 5;
/// Rows 4..12 hold the invader that is in range of the player's shots.
const INVADER_ROWS: std::ops::Range<usize> = 4..12;
/// Every invader of the first row is drawn with this many pixels.
const INVADER_PIXELS: usize = 34;

fn load() -> Option<Chip8<LinearCongruentialGenerator>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os("RSC8_GAME_ROM_DIR") {
        roots.push(PathBuf::from(dir));
    }
    roots.push(manifest.join(".."));
    let rom = roots
        .into_iter()
        .find_map(|root| fs::read(root.join(ROM)).ok())?;

    let mut chip8 = Chip8::new(LinearCongruentialGenerator { seed: 1 });
    chip8.load_fontset();
    chip8.load_rom(&rom).expect("ROM fits in memory");
    Some(chip8)
}

fn run_frames(chip8: &mut Chip8<LinearCongruentialGenerator>, frames: std::ops::Range<usize>) {
    for frame in frames {
        chip8.keypad = [false; KEYPAD_SIZE];
        if frame >= START_FRAME && (frame - START_FRAME) % FIRE_INTERVAL < FIRE_FRAMES {
            chip8.keypad[FIRE_KEY] = true;
        }
        chip8.fade_screen();
        for _ in 0..TICKS_PER_FRAME {
            chip8.tick().expect("game runs");
        }
        chip8.tick_timer();
    }
}

fn invader_pixels(chip8: &Chip8<LinearCongruentialGenerator>) -> usize {
    let (width, _) = chip8.screen_dimensions();
    INVADER_ROWS
        .map(|y| {
            (0..width)
                .filter(|x| chip8.screen_pixels()[y * width + x])
                .count()
        })
        .sum()
}

#[test]
fn shooting_an_invader_removes_only_that_invader() {
    let Some(mut chip8) = load() else {
        eprintln!("Space Invaders ROM not found; set RSC8_GAME_ROM_DIR to run this test");
        return;
    };

    run_frames(&mut chip8, 0..100);
    assert_eq!(
        invader_pixels(&chip8),
        4 * INVADER_PIXELS,
        "all four invaders of the first row are on screen"
    );

    run_frames(&mut chip8, 100..200);
    assert_eq!(
        invader_pixels(&chip8),
        3 * INVADER_PIXELS,
        "the first shot removes exactly one invader"
    );
}
