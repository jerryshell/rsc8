//! Runs the ROMs of [Timendus' CHIP-8 test suite] against the emulator.
//!
//! The ROMs are not vendored in this repository. The tests look for them in the
//! directory given by `RSC8_TEST_SUITE_DIR`, in a `chip8-test-suite` checkout
//! next to or inside this repository, and in the workspace root, in that order,
//! and are skipped when no ROM is found.
//!
//! Run with `UPDATE_SNAPSHOTS=1` to rewrite the screens in `tests/snapshots`.
//!
//! [Timendus' CHIP-8 test suite]: https://github.com/Timendus/chip8-test-suite

use rsc8_core::{
    chip8::{Chip8, KEYPAD_SIZE},
    rng::LinearCongruentialGenerator,
};

use std::{
    path::{Path, PathBuf},
    sync::Once,
};

const TICKS_PER_FRAME: usize = 8;
const SNAPSHOT_DIR: &str = "tests/snapshots";

/// The result glyphs the test suite draws with its `image-ok`, `image-no` and
/// `flag-ok`/`flag-err` bitmaps.
const RESULT_OK: [&str; 4] = ["....", "#.#.", "##..", "#..."];
const FLAG_OK: [&str; 3] = ["#.#.", "##..", "#..."];
const FLAG_ERR: [&str; 3] = ["#.#.", ".#..", "#.#."];

/// Result cells of the opcode test: one entry per column with its x position
/// and number of results, which the test draws from y=1 with a five pixel pitch.
const OPCODE_RESULT_COLUMNS: [(usize, usize); 4] = [(11, 6), (27, 6), (43, 6), (59, 4)];

fn rom(name: &str) -> Option<Vec<u8>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let roots = match std::env::var_os("RSC8_TEST_SUITE_DIR") {
        Some(dir) => vec![PathBuf::from(dir)],
        None => vec![
            manifest.join("../../chip8-test-suite"),
            manifest.join("../chip8-test-suite"),
            manifest.join(".."),
        ],
    };
    roots.into_iter().find_map(|root| {
        std::fs::read(root.join("bin").join(name))
            .or_else(|_| std::fs::read(root.join(name)))
            .ok()
    })
}

fn load(name: &str) -> Option<Chip8<LinearCongruentialGenerator>> {
    static MISSING: Once = Once::new();
    let Some(rom) = rom(name) else {
        MISSING.call_once(|| {
            eprintln!("CHIP-8 test suite not found; set RSC8_TEST_SUITE_DIR to run these tests");
        });
        return None;
    };
    let mut chip8 = Chip8::new(LinearCongruentialGenerator { seed: 1 });
    chip8.load_fontset();
    chip8.load_rom(&rom).expect("test suite ROM fits in memory");
    Some(chip8)
}

/// Runs the ROM the way the TUI does: the afterglow of the screen keeps fading,
/// `magic` is written to 0x1FF to preselect a menu option, and every key press
/// in `keys` is `(first frame, key, number of frames held)`.
fn run(
    mut chip8: Chip8<LinearCongruentialGenerator>,
    magic: Option<u8>,
    frames: usize,
    keys: &[(usize, usize, usize)],
) -> Chip8<LinearCongruentialGenerator> {
    chip8.memory[0x1FF] = magic.unwrap_or(0);
    for frame in 0..frames {
        chip8.fade_screen();
        chip8.keypad = keypad_of(frame, keys);
        for _ in 0..TICKS_PER_FRAME {
            if chip8.wait_for_key_release.is_some() {
                break;
            }
            chip8.tick().expect("test suite ROM runs");
        }
        chip8.tick_timer();
        if let Some(key) = chip8.wait_for_key_release
            && !chip8.keypad[key]
        {
            chip8.wait_for_key_release = None;
        }
    }
    chip8
}

fn keypad_of(frame: usize, keys: &[(usize, usize, usize)]) -> [bool; KEYPAD_SIZE] {
    let mut keypad = [false; KEYPAD_SIZE];
    for (start, key, frames) in keys {
        if (*start..start + frames).contains(&frame) {
            keypad[*key] = true;
        }
    }
    keypad
}

fn screen(chip8: &Chip8<LinearCongruentialGenerator>) -> String {
    let (width, height) = chip8.screen_dimensions();
    let pixels = chip8.screen_pixels();
    let mut screen = String::with_capacity((width + 1) * height);
    for y in 0..height {
        for x in 0..width {
            screen.push(if pixels[y * width + x] { '#' } else { '.' });
        }
        screen.push('\n');
    }
    screen
}

/// Returns whether `bitmap` is drawn at `(x, y)`. Bits are one character each.
fn bitmap_at(screen: &str, bitmap: &[&str], x: usize, y: usize) -> bool {
    let rows: Vec<&str> = screen.lines().collect();
    bitmap
        .iter()
        .enumerate()
        .all(|(dy, row)| rows.get(y + dy).and_then(|line| line.get(x..x + row.len())) == Some(*row))
}

fn bitmap_count(screen: &str, bitmap: &[&str]) -> usize {
    let rows: Vec<&str> = screen.lines().collect();
    let height = bitmap.len();
    let width = bitmap[0].len();
    let mut count = 0;
    for y in 0..rows.len().saturating_sub(height - 1) {
        for x in 0..rows[0].len().saturating_sub(width - 1) {
            if bitmap_at(screen, bitmap, x, y) {
                count += 1;
            }
        }
    }
    count
}

#[track_caller]
fn assert_snapshot(name: &str, chip8: &Chip8<LinearCongruentialGenerator>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(SNAPSHOT_DIR)
        .join(name);
    let actual = screen(chip8);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run UPDATE_SNAPSHOTS=1",
            path.display()
        )
    });
    assert_eq!(
        actual, expected,
        "screen of {name} changed; run UPDATE_SNAPSHOTS=1 and check the diff"
    );
}

const TEST_SUITE_ROMS: [&str; 8] = [
    "1-chip8-logo.ch8",
    "2-ibm-logo.ch8",
    "3-corax+.ch8",
    "4-flags.ch8",
    "5-quirks.ch8",
    "6-keypad.ch8",
    "7-beep.ch8",
    "8-scrolling.ch8",
];

#[test]
fn every_rom_runs_without_errors() {
    for name in TEST_SUITE_ROMS {
        let Some(chip8) = load(name) else {
            return;
        };
        run(chip8, None, 300, &[]);
    }
}

#[test]
fn every_scrolling_branch_runs_without_errors() {
    for magic in 1..=5 {
        let Some(chip8) = load("8-scrolling.ch8") else {
            return;
        };
        run(chip8, Some(magic), 400, &[]);
    }
}

#[test]
fn logos_are_drawn() {
    let Some(chip8) = load("1-chip8-logo.ch8") else {
        return;
    };
    assert_snapshot("1-chip8-logo.txt", &run(chip8, None, 300, &[]));

    let Some(chip8) = load("2-ibm-logo.ch8") else {
        return;
    };
    assert_snapshot("2-ibm-logo.txt", &run(chip8, None, 300, &[]));
}

#[test]
fn opcode_tests_pass() {
    let Some(chip8) = load("3-corax+.ch8") else {
        return;
    };
    let chip8 = run(chip8, None, 600, &[]);
    assert_snapshot("3-corax+.txt", &chip8);
    let screen = screen(&chip8);
    for (x, results) in OPCODE_RESULT_COLUMNS {
        for row in 0..results {
            let y = 1 + row * 5;
            assert!(
                bitmap_at(&screen, &RESULT_OK, x, y),
                "opcode test result at ({x}, {y}) is not OK"
            );
        }
    }
}

#[test]
fn flag_tests_pass() {
    let Some(chip8) = load("4-flags.ch8") else {
        return;
    };
    let chip8 = run(chip8, None, 600, &[]);
    assert_snapshot("4-flags.txt", &chip8);
    let screen = screen(&chip8);
    assert!(bitmap_count(&screen, &FLAG_OK) > 0);
    assert_eq!(bitmap_count(&screen, &FLAG_ERR), 0, "a flag test failed");
}

#[test]
fn quirks_match_the_chip8_platform() {
    let Some(mut chip8) = load("5-quirks.ch8") else {
        return;
    };
    // The CHIP-8 platform is the COSMAC VIP, which shifts VY into VX.
    chip8.shift_in_place = false;
    let chip8 = run(chip8, Some(1), 900, &[]);
    assert_snapshot("5-quirks.txt", &chip8);
    let screen = screen(&chip8);
    assert!(bitmap_count(&screen, &FLAG_OK) > 0);
    assert_eq!(
        bitmap_count(&screen, &FLAG_ERR),
        0,
        "a detected quirk does not match CHIP-8"
    );
}

#[test]
fn default_quirks_are_superchip_shifts() {
    let Some(chip8) = load("5-quirks.ch8") else {
        return;
    };
    let chip8 = run(chip8, Some(1), 900, &[]);
    let screen = screen(&chip8);
    // The default shifts VX in place, which is what most games expect, but the
    // COSMAC VIP platform shifts VY into VX instead and reports this one error.
    assert!(
        bitmap_at(&screen, &FLAG_ERR, 59, 22),
        "SHIFTING should differ"
    );
    assert_eq!(bitmap_count(&screen, &FLAG_ERR), 1);
}

#[test]
fn scrolling_tests_pass() {
    let cases = [
        (1, "8-scrolling-schip-lores.txt"),
        (3, "8-scrolling-schip-hires.txt"),
        (4, "8-scrolling-xochip-lores.txt"),
        (5, "8-scrolling-xochip-hires.txt"),
    ];
    for (magic, snapshot) in cases {
        let Some(chip8) = load("8-scrolling.ch8") else {
            return;
        };
        let chip8 = run(chip8, Some(magic), 400, &[]);
        assert_snapshot(snapshot, &chip8);
    }
}

#[test]
fn keypad_ex9e_shows_pressed_keys() {
    let Some(chip8) = load("6-keypad.ch8") else {
        return;
    };
    let idle = run(chip8, Some(1), 300, &[]);
    let Some(chip8) = load("6-keypad.ch8") else {
        return;
    };
    let pressed = run(chip8, Some(1), 300, &[(100, 1, 200), (100, 6, 200)]);
    assert_ne!(screen(&idle), screen(&pressed));
    assert_snapshot("6-keypad-ex9e.txt", &pressed);
}

#[test]
fn keypad_getkey_reports_all_good() {
    let Some(chip8) = load("6-keypad.ch8") else {
        return;
    };
    let chip8 = run(chip8, Some(3), 400, &[(100, 5, 10)]);
    assert_snapshot("6-keypad-getkey.txt", &chip8);
    let screen = screen(&chip8);
    assert!(
        bitmap_count(&screen, &FLAG_OK) > 0,
        "FX0A did not report success"
    );
    assert_eq!(
        bitmap_count(&screen, &FLAG_ERR),
        0,
        "FX0A reported an error"
    );
}
