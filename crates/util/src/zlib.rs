/*!
 * Routines for compressing and decompressing zlib-wrapped deflate messages
 *
 * These make use of persistent thread-local buffers to avoid repeated allocations.
 */

use miniz_oxide::deflate::core::{
    compress, create_comp_flags_from_zip_params, CompressorOxide, TDEFLFlush, TDEFLStatus,
};
use miniz_oxide::inflate::{
    core::{decompress, inflate_flags, DecompressorOxide},
    TINFLStatus,
};

use std::{cell::RefCell, error::Error, fmt};

pub use miniz_oxide::inflate::DecompressError as InflateError;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeflateError(TDEFLStatus);

impl Error for DeflateError {}

impl fmt::Display for DeflateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            TDEFLStatus::BadParam => "Bad Parameter",
            TDEFLStatus::PutBufFailed => "IO Error",
            TDEFLStatus::Okay | TDEFLStatus::Done => unreachable!(),
        })
    }
}

// Near exact copy of `miniz_oxide::deflate::compress_to_vec_inner` with thread-local compressor to reuse memory
pub fn deflate(input: &[u8], level: u8) -> Result<Vec<u8>, DeflateError> {
    thread_local! {
        static COMPRESSOR: RefCell<(u8, CompressorOxide)> = RefCell::new((7, CompressorOxide::new(create_comp_flags_from_zip_params(7, 1, 0))));
    }

    COMPRESSOR.with(|compressor| {
        let Ok(mut level_compressor) = compressor.try_borrow_mut() else {
            return Err(DeflateError(TDEFLStatus::BadParam));
        };

        let (ref mut old_level, ref mut compressor) = *level_compressor;

        compressor.reset();

        if *old_level != level {
            compressor.set_compression_level_raw(level);
            *old_level = level;
        }

        let mut output = vec![0; std::cmp::max(input.len() / 2, 2)];

        let mut in_pos = 0;
        let mut out_pos = 0;

        loop {
            #[rustfmt::skip]
            let (status, bytes_in, bytes_out) = compress(
                compressor,
                &input[in_pos..],
                &mut output[out_pos..],
                TDEFLFlush::Finish,
            );

            out_pos += bytes_out;
            in_pos += bytes_in;

            match status {
                TDEFLStatus::Done => {
                    output.truncate(out_pos);
                    return Ok(output);
                }
                TDEFLStatus::Okay => {
                    // We need more space, so resize the vector.
                    if output.len().saturating_sub(out_pos) < 30 {
                        output.resize(output.len() * 2, 0)
                    }
                }
                _ => return Err(DeflateError(status)),
            }
        }
    })
}

pub fn inflate(input: &[u8], limit: Option<usize>) -> Result<Vec<u8>, InflateError> {
    thread_local! {
        static DECOMPRESSOR: RefCell<DecompressorOxide> = RefCell::default();
    }

    const FLAGS: u32 =
        inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;

    DECOMPRESSOR.with(|decomp| {
        let Ok(mut decomp) = decomp.try_borrow_mut() else {
            return Err(InflateError {
                status: TINFLStatus::Failed,
                output: Vec::new(),
            });
        };

        decomp.init(); // reset decompressor

        let max_output_size = limit.unwrap_or(usize::MAX);

        let mut output: Vec<u8> = vec![0; input.len().saturating_mul(2).min(max_output_size)];

        let mut in_pos = 0;
        let mut out_pos = 0;

        loop {
            // Wrap the whole output slice so we know we have enough of the
            // decompressed data for matches.
            let (status, in_consumed, out_consumed) =
                decompress(&mut decomp, &input[in_pos..], &mut output, out_pos, FLAGS);
            in_pos += in_consumed;
            out_pos += out_consumed;

            match status {
                TINFLStatus::Done => {
                    output.truncate(out_pos);
                    return Ok(output);
                }

                TINFLStatus::HasMoreOutput => {
                    // if the buffer has already reached the size limit, return an error
                    if output.len() >= max_output_size {
                        return Err(InflateError { status, output });
                    }
                    // calculate the new length, capped at `max_output_size`
                    let new_len = output.len().saturating_mul(2).min(max_output_size);
                    output.resize(new_len, 0);
                }

                _ => return Err(InflateError { status, output }),
            }
        }
    })
}
