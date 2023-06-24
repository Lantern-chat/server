/*!
 * Probes a GIF to detect if it _actually_ has transparent pixels, and accumulates misc data while we're at it.
 *
 * The final algorithm for this is lightweight and simple, and only requires reading the first frame in full.
 *
 * For the common GIF, there are only two ways to obtain real transparency. It can either have transparent
 * pixels in the first frame, or clears parts of the image using the `Background` dispose method after a frame.
 * Technically, the `background` dispose method is supposed to fill in the frame with the background color,
 * but everyone ignores that.
 *
 * Therefore, it is not necessary to actually accumulate and dispose pixels values.
 *
 * Note: This binary intentionally has bad error handling. It either succeeds or it doesn't.
 * Any file that fails to process should be considered invalid.
 *
 * Usage:
 * ```
 * gif_probe path/file.gif
 *     [-l max_duration_in_ms]
 *     [-d max_pixels]
 *     [-m max_memory_in_bytes]
 * ```
 *
 * Output:
 * ```rust
 * pub struct GifProbe {
 *     pub alpha: bool,
 *     pub max_colors: u16,
 *     pub duration: u64,
 *     pub frames: u64,
 *     pub width: u16,
 *     pub height: u16,
 * }
 * ```
 */

use std::{io::Write, path::PathBuf};

use gif::{ColorOutput, DecodeOptions, DisposalMethod, MemoryLimit};

#[derive(Default, Debug, serde::Serialize)]
pub struct GifProbe {
    pub alpha: bool,
    pub max_colors: u16,
    pub duration: u64,
    pub frames: u64,
    pub width: u16,
    pub height: u16,
}

fn main() {
    let mut args = pico_args::Arguments::from_env();

    let Ok(path) = args.free_from_os_str(|s| PathBuf::try_from(s)) else {
        panic!("No input file given");
    };

    // stop processing after this duration is reached
    let max_duration = args.opt_value_from_str::<_, u64>("-l").unwrap().unwrap_or(u64::MAX);
    // panic if the given number of pixels is too many
    let max_pixels = args.opt_value_from_str::<_, u64>("-d").unwrap();
    // don't decode if the decoder would allocate more than this
    let max_memory = args.opt_value_from_str::<_, u32>("-m").unwrap();

    let f = std::fs::OpenOptions::new().read(true).write(false).open(&path).expect("To open the file");

    let mut opts = DecodeOptions::new();

    opts.set_memory_limit(MemoryLimit(max_memory.unwrap_or(1024 * 1024 * 20))); // 20 MiB or user-specified
    opts.set_color_output(ColorOutput::Indexed);
    opts.check_frame_consistency(true);
    opts.allow_unknown_blocks(false);
    opts.check_lzw_end_code(false);

    let mut d = opts.read_info(f).expect("To read the GIF");

    let mut probe = GifProbe {
        width: d.width(),
        height: d.height(),
        ..GifProbe::default()
    };

    if let Some(m) = max_pixels {
        if m < (probe.width as u64 * probe.height as u64) {
            panic!("Image too large!");
        }
    }

    if let Some(p) = d.global_palette() {
        probe.max_colors = u16::try_from(p.len() / 3).unwrap();
    }

    if let Some(frame) = d.read_next_frame().expect("to read the first frame") {
        if let Some(tr) = frame.transparent {
            if frame.buffer.contains(&tr) {
                probe.alpha = true;
            }
        }

        probe.frames += 1;
        probe.duration += frame.delay as u64;
        if let Some(ref p) = frame.palette {
            probe.max_colors = probe.max_colors.max(u16::try_from(p.len() / 3).unwrap());
        }
    }

    while let Some(frame) = d.next_frame_info().expect("to read the frame") {
        if frame.dispose == DisposalMethod::Background && frame.width > 0 && frame.height > 0 {
            probe.alpha = true;
        }

        probe.frames += 1;
        probe.duration += frame.delay as u64;
        if let Some(ref p) = frame.palette {
            probe.max_colors = probe.max_colors.max(u16::try_from(p.len() / 3).unwrap());
        }

        if probe.duration >= max_duration {
            break;
        }
    }

    let mut stdout = std::io::stdout();

    serde_json::to_writer(&mut stdout, &probe).unwrap();
    stdout.write_all(b"\n").unwrap();
}
