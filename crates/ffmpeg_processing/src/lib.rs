extern crate tracing as log;

use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use gcd::Gcd;
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio::{fs::File, process::Command};

pub mod probe;

#[derive(Debug, thiserror::Error)]
pub enum FfmpegError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("Missing Pipe")]
    MissingPipe,

    #[error("Missing File")]
    MissingFile,

    #[error("Not Video")]
    NotVideo,

    #[error("Needs Probe")]
    NeedsProbe,

    #[error("Probe Error")]
    ProbeError,

    #[error("Encode Error")]
    EncodeError,

    #[error("Json Error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub struct Ffmpeg {
    /// Path to where `ffmpeg` and `ffprobe` exist.
    ///
    /// Set this to empty to just use the system `ffmpeg`
    pub bin: PathBuf,

    /// Path for where to store unencrypted files temporarily
    ///
    /// Because they will be unencrypted, using a `tmpfs` is recommended.
    pub tmp: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Gif,
    WebM,
    Mp4,
}

impl Format {
    fn as_str(&self) -> &'static str {
        match self {
            Format::Gif => "gif",
            Format::WebM => "webm",
            Format::Mp4 => "mp4",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crop {
    Top,
    Middle,
    Bottom,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Encode {
    pub quality: u8,
    pub format: Format,
    pub crop: Crop,
    pub size: (u32, u32),
    /// Max frames per second
    pub max_fps: f32,
    /// Max length (in seconds)
    pub max_len: f32,
}

impl Default for Encode {
    fn default() -> Self {
        Encode {
            quality: 95,
            format: Format::Gif,
            crop: Crop::Center,
            size: (256, 256),
            max_fps: 24.0,
            max_len: 15.0,
        }
    }
}

impl Encode {
    #[rustfmt::skip]
    fn path(&self, name: &str) -> String {
        let Encode {
            quality, format, crop, size: (w, h), max_fps, max_len
        } = self;
        format!("{name}_{quality}_{crop:?}_{w}x{h}_{max_len}s_{max_fps}fps.{}", format.as_str())
    }
}

pub struct Input<'a, R> {
    ffmpeg: &'a Ffmpeg,
    name: &'a str,
    src: Option<R>,
    pub probe: Option<probe::FfProbeOutput>,
    len: u64,
    tmps: Vec<PathBuf>,
}

impl Ffmpeg {
    pub fn input<'a, 'b: 'a, R: AsyncRead + Unpin>(&'b self, name: &'a str, src: R) -> Input<'a, R> {
        Input {
            ffmpeg: self,
            name,
            src: Some(src),
            probe: None,
            len: 0,
            tmps: Vec::new(),
        }
    }

    fn ffprobe(&self) -> PathBuf {
        self.bin.join("ffprobe")
    }

    fn ffmpeg(&self) -> PathBuf {
        self.bin.join("ffmpeg")
    }

    fn tmp(&self, name: &str) -> PathBuf {
        self.tmp.join(name)
    }
}

impl<'a, R: AsyncRead + Unpin> Input<'a, R> {
    pub async fn probe(&mut self) -> Result<(), FfmpegError> {
        let Some(mut src) = self.src.take() else {
            return Err(FfmpegError::MissingFile);
        };

        let path = self.ffmpeg.tmp(self.name);

        let mut tmp = File::create(&path).await?;

        let mut ffprobe_cmd = Command::new(self.ffmpeg.ffprobe());
        ffprobe_cmd
            .args("-loglevel error -show_streams -show_format -print_format json -i".split_whitespace())
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        // after use in creating the command, and after the file was created,
        // push it to the tmps list for later cleanup
        self.tmps.push(path);

        self.len += tokio::io::copy(&mut src, &mut tmp).await?;
        tmp.flush().await?;

        let ffprobe = ffprobe_cmd.spawn()?;

        let out = ffprobe.wait_with_output().await?;

        if !out.status.success() {
            return Err(FfmpegError::ProbeError);
        }

        self.probe = Some(serde_json::from_slice(&out.stdout)?);

        Ok(())
    }

    pub async fn encode(&mut self, opts: Encode) -> Result<PathBuf, FfmpegError> {
        let Some(ref probe) = self.probe else {
            return Err(FfmpegError::NeedsProbe);
        };

        let Some(stream) = probe.streams.iter().find(|s| matches!(s.codec_type, Some(ref ct) if ct == "video")) else {
            return Err(FfmpegError::NotVideo);
        };

        let (sw, sh) = (stream.width.unwrap_or(0) as u64, stream.height.unwrap_or(0) as u64);

        let in_path = self.ffmpeg.tmp(self.name);
        let out_path: PathBuf = self.ffmpeg.tmp(&opts.path(self.name));

        let (w, h) = opts.size;
        let (aw, ah) = {
            let g = w.gcd(h);
            (w / g, h / g)
        };

        #[rustfmt::skip]
        let crop = match opts.crop {
            Crop::Center => format!("'min(iw,ih)':'min(ih,ow*{ah}/{aw})'"),
            Crop::Top    => format!("'min(iw,ih*{aw}/{ah})':ow*{ah}/{aw}:(iw-ow)/2:0"),
            Crop::Middle => format!("'min(iw,ih*{aw}/{ah})':ow*{ah}/{aw}"),
            Crop::Bottom => format!("'min(iw,ih*{aw}/{ah})':ow*{ah}/{aw}:(iw-ow)/2:ih-ow"),
        };

        let mut ffmpeg_args = format!(
            "-t {}s -fpsmax {} -fflags +bitexact -flags:v +bitexact -map_metadata -1 -vn -an -dn -sn ",
            opts.max_len, opts.max_fps
        );

        let mut filter = format!(
            "[v:{}]crop={crop},scale='min(iw,{w})':'min(ih,{h})':flags=lanczos:force_original_aspect_ratio=increase{}",
            stream.index, if opts.format == Format::Mp4 { ":force_divisible_by=2" } else { "" }
        );

        // GIFs are terrible, filter out dithering artifacts
        if matches!(stream.codec_name, Some(ref n) if n == "gif") {
            let block_size = w.min(h).ilog2().next_power_of_two().clamp(8, 256);

            filter += &format!(
                ",atadenoise=0.3:0.04:0.3:0.04:0.3:0.04:s=9,\
                fftdnoiz=sigma=15:block={block_size}:overlap=0.8[c];"
            );
        } else {
            filter += "[c];"; // otherwise just end filter node here
        }

        match opts.format {
            Format::Gif => {
                // GIF lossy score
                let mut lossy = 100u8.saturating_sub(opts.quality) as u32;
                if opts.quality < 90 {
                    lossy = lossy * 3 - 20;
                }

                let max_colors = match opts.quality {
                    00..=39 => 64,
                    40..=59 => 96,
                    60..=79 => 128,
                    80..=89 => 128 + 64,
                    90.. => 256,
                };

                let (dither, scale) = match () {
                    // low-quality
                    _ if opts.quality <= 50 => ("none", 0),
                    // if the output is large
                    _ if w.max(h) >= 512 => ("bayer", 5),
                    // if the input is not small
                    _ if sw > 256 && sh > 256 => ("bayer", 3),
                    // if the input is very small
                    _ if sw <= 64 || sh <= 64 => ("none", 0),

                    _ => ("sierra2_4a", 0),
                };

                filter += &format!(
                    "[c]mpdecimate,split[a][b];\
                        [a]palettegen=max_colors={max_colors}:stats_mode=diff[p];\
                        [b][p]paletteuse=dither={dither}:bayer_scale={scale}:diff_mode=rectangle:alpha_threshold=128"
                );

                let mut ffmpeg: Command = Command::new(self.ffmpeg.ffmpeg());
                ffmpeg
                    .args("-loglevel error -y -i".split_whitespace())
                    .arg(&in_path)
                    .args(ffmpeg_args.split_whitespace())
                    .args(["-filter_complex", &filter])
                    .args(["-f", "gif"])
                    .arg(&out_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .kill_on_drop(true);

                let mut gifsicle = Command::new("gifsicle");
                gifsicle
                    .args(format!("--conserve-memory -O3 --lossy={lossy} -b -i").split_whitespace())
                    .arg(&out_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .kill_on_drop(true);

                log::debug!("{ffmpeg:?} | {gifsicle:?}");

                let mut ffmpeg = ffmpeg.spawn()?;

                self.tmps.push(out_path.clone());

                if !ffmpeg.wait().await?.success() {
                    return Err(FfmpegError::EncodeError);
                }

                let mut gifsicle = gifsicle.spawn()?;

                if !gifsicle.wait().await?.success() {
                    return Err(FfmpegError::EncodeError);
                }

                // check if the original was suitable for direct use in the first place
                // and return that instead if it's smaller
                'check: {
                    if !matches!(stream.codec_name, Some(ref n) if n == "gif") {
                        break 'check;
                    }

                    let g = sw.gcd(sh);
                    let (asw, ash) = ((sw / g) as u32, (sh / g) as u32);

                    if asw != aw || ash != ah {
                        break 'check;
                    }

                    let len = tokio::fs::metadata(&out_path).await?.len();

                    if len > self.len {
                        return Ok(in_path);
                    }
                };

                Ok(out_path)
            }
            Format::WebM | Format::Mp4 => {
                if Format::WebM == opts.format {
                    filter += "[c]copy";
                    ffmpeg_args += "-c:v libvpx-vp9 -frame-parallel 0 -cpu-used 1 -quality good -auto-alt-ref 0 ";
                } else {
                    // overlay it on a black background to remove any transparency
                    filter += "color=black[k];[k][c]scale2ref[k][c];[k][c]overlay=format=auto:shortest=1,setsar=1";
                    // specify a decent crf here to shrink file some, as it doesn't need to be high-quality
                    ffmpeg_args += "-c:v libx264 -pix_fmt yuv420p -crf 24 -bufsize 14000 -profile:v main \
                                    -level 3.1 -preset slow -x264-params ref=4 ";
                }

                let mut q = match opts.quality {
                    00..=14 => 36,
                    15..=39 => 32,
                    40..=79 => 30,
                    80..=89 => 26,
                    90.. => 24,
                };

                // VP9 is more efficient
                if Format::WebM == opts.format {
                    q += 6;
                }

                q -= 16 - (w.max(h).clamp(64, 512) - 64) * 16 / (512 - 64);

                ffmpeg_args += &format!("-movflags +faststart -qmin {q} -qmax {} ", q + 20);

                let mut has_br = false;

                // if the old codec was similar, we can probably do better than that, so constrain the bitrate to the original
                if matches!(stream.codec_name, Some(ref n) if ["h264", "h265", "vp9", "vp8"].contains(&n.as_str()))
                {
                    if let Some(br) = stream.max_bit_rate.as_ref().or(stream.bit_rate.as_ref()) {
                        if let Ok(br) = br.parse::<u64>() {
                            ffmpeg_args += &format!("-b:v {br} -maxrate {br} ");
                            has_br = true;
                        }
                    }
                }

                if !has_br {
                    let min = 128;
                    let max = 512;

                    let br = opts.quality as u32 * (max - min) / 100 + min;

                    ffmpeg_args += &format!("-minrate {min}k -b:v {br}k -maxrate {max}k ");
                }

                let mut ffmpeg: Command = Command::new(self.ffmpeg.ffmpeg());
                ffmpeg
                    .args("-y -i".split_whitespace())
                    .arg(&in_path)
                    .args(["-filter_complex", &filter])
                    .args(ffmpeg_args.split_whitespace())
                    .arg(&out_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .kill_on_drop(true);

                log::debug!("{ffmpeg:?}");

                let mut ffmpeg = ffmpeg.spawn()?;

                self.tmps.push(out_path.clone());

                if !ffmpeg.wait().await?.success() {
                    return Err(FfmpegError::EncodeError);
                }

                Ok(out_path)
            }
        }
    }
}

impl<R> Drop for Input<'_, R> {
    fn drop(&mut self) {
        let tmps = std::mem::take(&mut self.tmps);

        tokio::task::spawn_blocking(move || {
            for path in tmps {
                let _ = std::fs::remove_file(&path);
            }
        });
    }
}
