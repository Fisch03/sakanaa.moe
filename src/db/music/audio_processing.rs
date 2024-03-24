use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

use super::types::{BeatEvent, BeatEventType};

use symphonia::core::{
    audio::SampleBuffer,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataRevision, StandardTagKey, Value},
    probe::Hint,
};

use mp3lame_encoder::{Builder, FlushNoGap, InterleavedPcm, MonoPcm};

use aubio_rs::{Smpl, Tempo};
const I16_TO_SMPL: Smpl = 1.0 / (1 << 16) as Smpl;

#[derive(Debug, Default)]
pub struct Metadata {
    pub track: Option<String>,
    pub mb_track: Option<String>,

    pub artist: Option<String>,
    pub mb_artist: Option<String>,

    pub album_artist: Option<String>,
    pub mb_album_artist: Option<String>,

    pub album: Option<String>,
    pub mb_album: Option<String>,
}
impl From<&MetadataRevision> for Metadata {
    fn from(revision: &MetadataRevision) -> Self {
        let mut metadata = Self::default();

        revision.tags().iter().for_each(|tag| {
            if let Some(key) = tag.std_key {
                let value = match &tag.value {
                    Value::String(value) => value.to_string(),
                    _ => return,
                };

                match key {
                    StandardTagKey::TrackTitle => metadata.track = Some(value),
                    StandardTagKey::MusicBrainzTrackId => metadata.mb_track = Some(value),

                    StandardTagKey::Artist => metadata.artist = Some(value),
                    StandardTagKey::MusicBrainzArtistId => metadata.mb_artist = Some(value),

                    StandardTagKey::AlbumArtist => metadata.album_artist = Some(value),
                    StandardTagKey::MusicBrainzAlbumArtistId => {
                        metadata.mb_album_artist = Some(value)
                    }

                    StandardTagKey::Album => metadata.album = Some(value),
                    StandardTagKey::MusicBrainzAlbumId => metadata.mb_album = Some(value),
                    _ => {}
                }
            }
        });

        metadata
    }
}

pub fn get_metadata<P: AsRef<Path>>(path: P) -> Result<Metadata> {
    let file = std::fs::File::open(path)?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());

    let hint: Hint = Default::default();
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let mut probed =
        symphonia::default::get_probe().format(&hint, stream, &fmt_opts, &meta_opts)?;

    let mut format = probed.format;

    let mut found_metadata = None;
    if let Some(mut metadata) = probed.metadata.get() {
        if let Some(revision) = metadata.skip_to_latest() {
            dbg!(&revision);
            found_metadata = Some(Metadata::from(revision));
        }
    }

    if found_metadata.is_none() {
        if let Some(metadata) = format.metadata().current() {
            found_metadata = Some(Metadata::from(metadata));
        }
    }

    Ok(found_metadata.context("No metadata found")?)
}

#[derive(Debug, Serialize)]
pub struct ProcessedAudio {
    pub mp3_data: Vec<u8>,
    pub beat_data: Vec<BeatEvent>,
}

#[derive(Debug)]
struct BPMCluster {
    values: Vec<f32>,
    total: f64,
    average: f32,
}

const BPM_CLUSTER_THRESHOLD: u32 = 8;
impl BPMCluster {
    fn new() -> Self {
        Self {
            values: Vec::with_capacity(1024),
            total: 0.0,
            average: 0.0,
        }
    }
    fn add(&mut self, bpm: f32) {
        self.values.push(bpm);
        self.total += bpm as f64;
        self.average = (self.total / self.values.len() as f64) as f32;
    }
    fn median(&mut self) -> f32 {
        self.values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = self.values.len() / 2;
        if self.values.len() % 2 == 0 {
            (self.values[mid] + self.values[mid - 1]) / 2.0
        } else {
            self.values[mid]
        }
    }
}

pub fn analyze_file<P: AsRef<Path>>(path: P) -> Result<ProcessedAudio> {
    let file = std::fs::File::open(path)?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());

    let hint: Hint = Default::default();
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe().format(&hint, stream, &fmt_opts, &meta_opts)?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| {
            t.codec_params.codec != CODEC_TYPE_NULL
                && if let Some(channels) = t.codec_params.channels {
                    (1..3).contains(&channels.count())
                } else {
                    false
                }
        })
        .context("No suitable track found")?;
    let codec_params = &track.codec_params;

    let dec_opts: DecoderOptions = Default::default();

    let mut decoder = symphonia::default::get_codecs().make(codec_params, &dec_opts)?;
    let mut sample_buf = None;

    let track_id = track.id;

    let num_channels = codec_params.channels.unwrap().count(); // we already checked the number of
                                                               // channels in finding the track so
                                                               // unwrapping is fine.
    let num_frames = codec_params.n_frames.context("No frames on track")?;
    let sample_rate = codec_params
        .sample_rate
        .context("No sample rate on track")?;
    let num_samples = num_channels * num_frames as usize;

    let mut mp3_encoder = Builder::new().context("Failed to create mp3 encoder")?;
    mp3_encoder
        .set_num_channels(num_channels.try_into().unwrap())
        .expect("Failed to set num channels");
    mp3_encoder
        .set_sample_rate(sample_rate)
        .expect("Failed to set sample rate");

    let mut mp3_encoder = mp3_encoder.build().expect("Failed to build mp3 encoder");
    let mut mp3_out_buf = Vec::new();
    mp3_out_buf.reserve(mp3lame_encoder::max_required_buffer_size(num_samples));

    let tempo_buffer_size = 1024;
    let tempo_hop_size = 512;
    let mut tempo_block = Vec::with_capacity(256);
    let mut tempo = Tempo::new(
        aubio_rs::OnsetMode::SpecFlux,
        tempo_buffer_size,
        tempo_hop_size,
        sample_rate,
    )?;

    let mut bpm_clusters: Vec<BPMCluster> = Vec::new();
    let mut bpm_events: Vec<(u64, usize)> = Vec::new();
    let mut current_cluster: usize = 0;

    let start = std::time::Instant::now();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => {
                break;
            }
        };

        while !format.metadata().is_latest() {
            format.metadata().pop();
        }

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                if sample_buf.is_none() {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity() as u64;
                    sample_buf = Some(SampleBuffer::<i16>::new(duration, spec));
                }

                if let Some(buf) = &mut sample_buf {
                    let encoded_size;

                    buf.copy_interleaved_ref(decoded);

                    let mut samples = buf.samples();
                    while samples.len() >= tempo_buffer_size {
                        let mut taken = 0;
                        tempo_block.extend(
                            samples[..tempo_buffer_size]
                                .iter()
                                .take(tempo_buffer_size - tempo_block.len())
                                .map(|sample| {
                                    taken += 1;
                                    *sample as Smpl * I16_TO_SMPL
                                }),
                        );
                        samples = &samples[taken..];

                        if tempo_block.len() < tempo_buffer_size {
                            continue;
                        }

                        tempo.do_result(&tempo_block)?;
                        let bpm = tempo.get_bpm();
                        let existing_cluster_pos = bpm_clusters
                            .iter_mut()
                            .position(|c| (c.average - bpm).abs() < BPM_CLUSTER_THRESHOLD as f32);
                        let existing_cluster =
                            existing_cluster_pos.map(|pos| &mut bpm_clusters[pos]);

                        let time_ms =
                            (tempo.get_last() as f64 / sample_rate as f64 * 1000.0).round() as u64;

                        if tempo.get_confidence() > 0.05 {
                            if let Some(cluster) = existing_cluster {
                                cluster.add(bpm);
                                if current_cluster != existing_cluster_pos.unwrap() {
                                    current_cluster = existing_cluster_pos.unwrap();
                                    //println!("Switching to BPM Cluster: {}", bpm);

                                    bpm_events.push((time_ms, current_cluster));
                                }
                            } else {
                                let mut cluster = BPMCluster::new();
                                cluster.add(bpm);
                                bpm_clusters.push(cluster);
                                current_cluster = bpm_clusters.len() - 1;
                                /*
                                println!(
                                    "New BPM Cluster: {}, confidence: {}",
                                    bpm,
                                    tempo.get_confidence()
                                );
                                */

                                bpm_events.push((time_ms, bpm_clusters.len() - 1));
                            };
                        }
                        tempo_block.clear();
                    }

                    if num_channels == 1 {
                        let mono_pcm = MonoPcm(&buf.samples());
                        encoded_size = mp3_encoder
                            .encode(mono_pcm, mp3_out_buf.spare_capacity_mut())
                            .expect("Failed to encode");
                    } else {
                        let interleaved_pcm = InterleavedPcm(&buf.samples());
                        encoded_size = mp3_encoder
                            .encode(interleaved_pcm, mp3_out_buf.spare_capacity_mut())
                            .expect("Failed to encode");
                    }

                    unsafe { mp3_out_buf.set_len(mp3_out_buf.len().wrapping_add(encoded_size)) }
                }
            }
            Err(_) => {
                break;
            }
        }
    }

    let encoded_size = mp3_encoder
        .flush::<FlushNoGap>(mp3_out_buf.spare_capacity_mut())
        .expect("Failed to flush");
    unsafe { mp3_out_buf.set_len(mp3_out_buf.len().wrapping_add(encoded_size)) }

    println!(
        "\nEncoding took: {:?}. Found {} Events",
        start.elapsed(),
        bpm_events.len()
    );

    let first_event = bpm_events.first().unwrap();
    let first_event_bpm = bpm_clusters[first_event.1].median();
    let ms_per_beat = 60_000.0 / first_event_bpm as f64;
    let first_possible_beat = first_event.0 as f64 % ms_per_beat as f64;

    let mut beat_data: Vec<BeatEvent> = bpm_events
        .into_iter()
        .map(|(time_ms, cluster)| BeatEvent {
            time_ms,
            event_type: BeatEventType::BPM(bpm_clusters[cluster].median()),
        })
        .collect();

    beat_data.get_mut(0).unwrap().time_ms = first_possible_beat as u64;

    // the bpm estimation really doesn't like giving out high bpms, so if all of the bpms are low
    // then we assume it should be doubled. if the listened music is expected to be less than 200bpm
    // most of the time, this part should be removed
    let all_slow = beat_data.iter().all(|b| match &b.event_type {
        BeatEventType::BPM(bpm) => *bpm < 200.0 && bpm * 2.0 < 300.0,
    });
    if all_slow {
        beat_data.iter_mut().for_each(|b| match &mut b.event_type {
            BeatEventType::BPM(bpm) => *bpm *= 2.0,
        });
    }

    Ok(ProcessedAudio {
        mp3_data: mp3_out_buf,
        beat_data,
    })
}
