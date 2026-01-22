use std::{collections::HashMap, ops::Div, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ClipManager {
    pub clips: HashMap<(usize, usize), Clip>,
}

impl ClipManager {
    pub const PEAK_BUCKET_SIZE: usize = 256;

    pub fn import(&mut self, showfile: PathBuf) -> Result<(), std::io::Error> {
        let read_channel_dir = std::fs::read_dir(showfile.join("playback_media"))?;
        for channel_dir_res in read_channel_dir {
            let channel_dir = channel_dir_res?;
            let read_clip_dir = std::fs::read_dir(channel_dir.path())?;
            for clip_file_res in read_clip_dir {
                let clip_file = clip_file_res?;

                let res = hound::WavReader::open(clip_file.path());
                if res.is_err() {
                    continue;
                }
                let mut reader = res.expect("Error case handled above.");

                let channel_idx: usize =
                    str::parse(channel_dir.file_name().to_str().unwrap_or("nope"))
                        .unwrap_or(usize::MAX);
                let clip_idx: usize = str::parse(
                    clip_file
                        .file_name()
                        .to_str()
                        .unwrap_or("nope.wav")
                        .split_once(".")
                        .unwrap_or(("nope", "wav"))
                        .0,
                )
                .unwrap_or(usize::MAX);

                if clip_idx == usize::MAX || channel_idx == usize::MAX {
                    continue;
                }

                let buf: Vec<f32> = match reader.spec().sample_format {
                    hound::SampleFormat::Float => reader
                        .samples::<f32>()
                        .map(|sample| {
                            if let Err(err) = sample {
                                return 0.0;
                            }
                            return sample.expect("Err already handled.");
                        })
                        .collect(),
                    hound::SampleFormat::Int => reader
                        .samples::<i32>()
                        .map(|sample| {
                            if let Err(err) = sample {
                                return 0.0;
                            }
                            return (sample.expect("Err already handled.") as f32).div(32768.0);
                        })
                        .collect(),
                };

                let mut clip = Clip::new(clip_file.path(), Self::PEAK_BUCKET_SIZE);
                clip.generate_peaks(buf);
                self.clips.insert((channel_idx, clip_idx), clip);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Clip {
    peak_bucket_size: usize,
    pub peak_buckets: Vec<f32>,
    pub path: PathBuf,
    pub length: usize,
}

impl Clip {
    pub fn new(path: PathBuf, peak_bucket_size: usize) -> Self {
        Self {
            peak_bucket_size,
            peak_buckets: vec![],
            path,
            length: 0,
        }
    }

    pub fn generate_peaks(&mut self, buf: Vec<f32>) {
        self.peak_buckets.clear();
        let mut sample_idx = 0;
        for _ in 0..(buf.len() / self.peak_bucket_size + 1) {
            self.peak_buckets.push(0.0);
        }
        while sample_idx < buf.len() {
            let peak_bucket_idx = sample_idx / self.peak_bucket_size;
            self.peak_buckets[peak_bucket_idx] += buf[sample_idx] / self.peak_bucket_size as f32;
            sample_idx += 1;
        }
        self.length = sample_idx;
        self.normalize();
        self.inflate();
    }

    pub fn normalize(&mut self) {
        let max_amplitude = self
            .peak_buckets
            .iter()
            .max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap())
            .unwrap_or(&1.0);
        self.peak_buckets = self
            .peak_buckets
            .iter()
            .map(|f| f / max_amplitude)
            .collect();
    }

    pub fn inflate(&mut self) {
        self.peak_buckets = self.peak_buckets.iter().map(|f| f.abs().cbrt()).collect();
    }
}
