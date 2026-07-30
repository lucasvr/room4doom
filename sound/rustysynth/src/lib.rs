//! Sound backend built on a software mixer and the rustysynth MIDI
//! synthesizer, for platforms without a system audio stack.
//!
//! The backend is hardware-agnostic: before the game constructs the sound
//! server, the embedding application registers an [`AudioSink`] through
//! [`register_audio`], along with an optional SF2 SoundFont used for music
//! synthesis (music is disabled when no SoundFont is provided). Sound effects
//! and MUS/MIDI music are mixed in software on a dedicated thread into
//! interleaved stereo `i16` blocks at [`OUTPUT_RATE`] Hz and pushed to the
//! sink. If no sink is registered the server consumes commands silently.

use std::error::Error;
use std::fmt::Display;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ::rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use log::{debug, info, warn};
use sound_traits::{InitResult, SfxName, SoundAction, SoundServer, SoundServerTic, MUS_DATA};
use wad::WadData;

use crate::info::SFX_INFO_BASE;
use crate::mus2midi::read_mus_to_midi;

/// Sample rate of the blocks pushed to the [`AudioSink`], in Hz.
pub const OUTPUT_RATE: u32 = 22_050;

/// Frames per mixed block (one frame is a left/right sample pair).
const BLOCK_FRAMES: usize = 512;

/// Number of simultaneously playing sound effects. Half of the SDL2
/// backend's 32 channels, sized for the embedded CPU budget.
const MIX_CHANNELS: usize = 16;

/// How far ahead of real time the mixer runs, in frames. Larger values
/// survive scheduling hiccups; smaller values reduce sound-effect latency.
const LEAD_FRAMES: u64 = 1536;

/// Sounds farther than this from the listener are inaudible.
const MAX_DIST: f32 = 1666.0;

/// Maximum polyphony of the music synthesizer.
const MUSIC_POLYPHONY: usize = 32;

/// Master volume of the music synthesizer (rustysynth defaults to 0.5,
/// which is much quieter than DOOM's near-full-scale sound effects).
/// Raise or lower this to rebalance music against sound effects.
const MUSIC_MASTER_VOLUME: f32 = 2.5;

/// Volume values received over [`SoundAction`] range from 0 to this.
const MAX_VOLUME: i32 = 128;

/// Fixed per-channel attenuation for sound effects, leaving headroom when
/// several near-full-scale effects and music overlap. Matches the SDL2
/// backend, which sets every chunk to half volume at load time.
const SFX_HEADROOM: f32 = 0.5;

const MUS_ID: [u8; 4] = [b'M', b'U', b'S', 0x1A];
const MID_ID: [u8; 4] = [b'M', b'T', b'h', b'd'];

pub type SndServerRx = Receiver<SoundAction<SfxName, usize>>;
pub type SndServerTx = Sender<SoundAction<SfxName, usize>>;

/// Where the mixed audio goes. Implemented by the embedding application on
/// top of its audio device.
pub trait AudioSink: Send {
    /// Push a block of interleaved stereo `i16` samples at [`OUTPUT_RATE`] Hz.
    /// Expected to block until the device has accepted the samples.
    fn play(&mut self, samples: &[i16]);

    /// Frames accepted by [`Self::play()`] but not yet played, or `None`
    /// when the sink cannot report it.
    fn queued_frames(&self) -> Option<u64> {
        None
    }

    /// Called once from the mixer thread before playback begins.
    fn configure_mixer_thread(&mut self) {}
}

struct AudioSetup {
    sink: Box<dyn AudioSink>,
    soundfont: Option<Vec<u8>>,
}

static AUDIO_SETUP: Mutex<Option<AudioSetup>> = Mutex::new(None);

/// Register the audio output used by the next `Snd` instance. Must be called
/// before `Game::new()` for sound to be audible.
pub fn register_audio(sink: Box<dyn AudioSink>, soundfont: Option<Vec<u8>>) {
    *AUDIO_SETUP.lock().unwrap() = Some(AudioSetup { sink, soundfont });
}

#[derive(Debug)]
pub enum SndError {
    None,
}

impl Display for SndError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Error for SndError {}

/// A decoded DMX sound-effect lump: unsigned 8-bit mono samples.
struct SfxSample {
    rate: u32,
    data: Vec<u8>,
    priority: i32,
}

#[derive(Debug, Default, Clone, Copy)]
struct Listener {
    uid: usize,
    x: f32,
    y: f32,
    angle: f32,
}

#[derive(Default, Clone, Copy)]
struct Channel {
    active: bool,
    uid: usize,
    sfx: usize,
    /// Position in the source sample data, 16.16 fixed point.
    pos: u64,
    /// Source samples per output frame, 16.16 fixed point.
    step: u32,
    left: f32,
    right: f32,
    priority: i32,
    x: f32,
    y: f32,
}

/// Per-channel stereo gains for a source at `(x, y)`, or `None` when the
/// source is out of hearing range. Sounds without an owner (uid 0) and the
/// listener's own sounds play centered at full volume.
fn channel_gains(listener: &Listener, uid: usize, x: f32, y: f32) -> Option<(f32, f32)> {
    if uid == 0 || uid == listener.uid {
        return Some((1.0, 1.0));
    }
    let dx = x - listener.x;
    let dy = y - listener.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist >= MAX_DIST {
        return None;
    }
    let atten = 1.0 - dist / MAX_DIST;
    // Bearing relative to the listener's facing direction; positive = left.
    let pan = (dy.atan2(dx) - listener.angle).sin();
    // Constant-power panning: centered sources play at -3 dB per side.
    let theta = (1.0 - pan) * core::f32::consts::FRAC_PI_4;
    Some((atten * theta.cos(), atten * theta.sin()))
}

fn volume_to_gain(volume: i32) -> f32 {
    volume.clamp(0, MAX_VOLUME) as f32 / MAX_VOLUME as f32
}

struct Mixer {
    sfx: Vec<SfxSample>,
    channels: [Channel; MIX_CHANNELS],
    listener: Listener,
    sfx_gain: f32,
    mus_gain: f32,
    music: Option<MidiFileSequencer>,
    music_paused: bool,
    // Scratch buffers used by `mix_block`.
    acc_left: Vec<f32>,
    acc_right: Vec<f32>,
}

impl Mixer {
    fn new(sfx: Vec<SfxSample>) -> Self {
        Self {
            sfx,
            channels: [Channel::default(); MIX_CHANNELS],
            listener: Listener::default(),
            sfx_gain: 1.0,
            mus_gain: 1.0,
            music: None,
            music_paused: false,
            acc_left: vec![0.0; BLOCK_FRAMES],
            acc_right: vec![0.0; BLOCK_FRAMES],
        }
    }

    fn start_sound(&mut self, uid: usize, sfx: SfxName, x: f32, y: f32) {
        let Some((left, right)) = channel_gains(&self.listener, uid, x, y) else {
            return;
        };
        let sample = &self.sfx[sfx as usize];
        if sample.data.is_empty() {
            return;
        }
        let (rate, priority) = (sample.rate, sample.priority);

        // A source only emits one sound at a time.
        self.stop_sound(uid);

        let channel = Channel {
            active: true,
            uid,
            sfx: sfx as usize,
            pos: 0,
            step: (((rate as u64) << 16) / OUTPUT_RATE as u64) as u32,
            left,
            right,
            priority,
            x,
            y,
        };

        if let Some(c) = self.channels.iter_mut().find(|c| !c.active) {
            *c = channel;
        } else if let Some(c) = self
            .channels
            .iter_mut()
            .find(|c| channel.priority >= c.priority)
        {
            *c = channel;
        }
    }

    fn update_listener(&mut self, uid: usize, x: f32, y: f32, angle: f32) {
        self.listener = Listener { uid, x, y, angle };
        for c in self.channels.iter_mut().filter(|c| c.active) {
            match channel_gains(&self.listener, c.uid, c.x, c.y) {
                Some((left, right)) => {
                    c.left = left;
                    c.right = right;
                }
                None => c.active = false,
            }
        }
    }

    fn stop_sound(&mut self, uid: usize) {
        for c in self.channels.iter_mut() {
            if c.uid == uid {
                *c = Channel::default();
            }
        }
    }

    fn stop_sound_all(&mut self) {
        self.channels = [Channel::default(); MIX_CHANNELS];
    }

    /// Mix all active channels and the music stream into `out` (interleaved
    /// stereo `i16`).
    fn mix_block(&mut self, out: &mut [i16]) {
        let frames = out.len() / 2;
        let acc_left = &mut self.acc_left[..frames];
        let acc_right = &mut self.acc_right[..frames];

        // Release a non-looping sequencer once it has played to the end.
        if matches!(&self.music, Some(s) if s.end_of_sequence()) {
            self.music = None;
        }

        match &mut self.music {
            Some(sequencer) if !self.music_paused => {
                sequencer.render(acc_left, acc_right);
                for s in acc_left.iter_mut().chain(acc_right.iter_mut()) {
                    *s *= self.mus_gain;
                }
            }
            _ => {
                acc_left.fill(0.0);
                acc_right.fill(0.0);
            }
        }

        for c in self.channels.iter_mut().filter(|c| c.active) {
            let sample = &self.sfx[c.sfx];
            let gain_l = c.left * self.sfx_gain * SFX_HEADROOM;
            let gain_r = c.right * self.sfx_gain * SFX_HEADROOM;
            for i in 0..frames {
                let idx = (c.pos >> 16) as usize;
                if idx >= sample.data.len() {
                    *c = Channel::default();
                    break;
                }
                let s = (sample.data[idx] as f32 - 127.5) / 127.5;
                acc_left[i] += s * gain_l;
                acc_right[i] += s * gain_r;
                c.pos += c.step as u64;
            }
        }

        for i in 0..frames {
            out[2 * i] = (acc_left[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            out[2 * i + 1] = (acc_right[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        }
    }
}

/// The mixer thread: keeps the sink topped up to [`LEAD_FRAMES`] of buffered
/// audio, enough that short scheduling delays don't underrun the device but
/// little enough that sound effects don't lag the action.
fn run_mixer(mixer: Arc<Mutex<Mixer>>, mut sink: Box<dyn AudioSink>, running: Arc<AtomicBool>) {
    sink.configure_mixer_thread();

    let block_time = Duration::from_micros(BLOCK_FRAMES as u64 * 1_000_000 / OUTPUT_RATE as u64);
    let mut block = vec![0i16; BLOCK_FRAMES * 2];

    // Some devices only begin playback once their buffer first fills (the
    // SAI DMA ring is one). Push silence until a write blocks, which is the
    // signal that playback is running; fill-based pacing then drains the
    // buffer to the intended lead and holds it there.
    if sink.queued_frames().is_some() {
        for _ in 0..1024 {
            if !running.load(Ordering::Relaxed) {
                return;
            }
            let before = Instant::now();
            sink.play(&block);
            if before.elapsed() > block_time / 2 {
                break;
            }
        }
    }

    // Wall-clock pacing state, used only when the sink can't report a level.
    let start = Instant::now();
    let mut pushed: u64 = 0;

    while running.load(Ordering::Relaxed) {
        let topped_up = match sink.queued_frames() {
            Some(queued) => queued >= LEAD_FRAMES,
            None => {
                let elapsed = start.elapsed().as_micros() as u64 * OUTPUT_RATE as u64 / 1_000_000;
                pushed >= elapsed + LEAD_FRAMES
            }
        };
        if topped_up {
            std::thread::sleep(block_time / 2);
            continue;
        }
        mixer.lock().unwrap().mix_block(&mut block);
        sink.play(&block);
        pushed += BLOCK_FRAMES as u64;
    }
}

/// Decode a DMX format sound-effect lump into (sample rate, samples).
fn decode_sfx_lump(lump: &[u8]) -> Option<(u32, Vec<u8>)> {
    if lump.len() < 8 {
        return None;
    }
    let rate = u16::from_le_bytes([lump[2], lump[3]]) as u32;
    let rate = if rate == 0 { 11_025 } else { rate };
    let len = u32::from_le_bytes([lump[4], lump[5], lump[6], lump[7]]) as usize;
    let data = &lump[8..8 + len.min(lump.len() - 8)];
    // Vanilla DMX lumps pad the sample data with 16 bytes on each side;
    // anything shorter than the padding is malformed.
    if data.len() <= 48 {
        return None;
    }
    Some((rate, data[16..data.len() - 16].to_vec()))
}

pub struct Snd {
    rx: SndServerRx,
    tx: SndServerTx,
    mixer: Option<Arc<Mutex<Mixer>>>,
    mixer_thread: Option<std::thread::JoinHandle<()>>,
    soundfont: Option<Arc<SoundFont>>,
    running: Arc<AtomicBool>,
    sfx_vol: i32,
    mus_vol: i32,
}

impl Snd {
    pub fn new(wad: &WadData) -> Result<Self, Box<dyn Error>> {
        let (tx, rx) = channel();
        let running = Arc::new(AtomicBool::new(true));

        let Some(setup) = AUDIO_SETUP.lock().unwrap().take() else {
            info!("No audio sink registered: sound disabled");
            return Ok(Self {
                rx,
                tx,
                mixer: None,
                mixer_thread: None,
                soundfont: None,
                running,
                sfx_vol: 0,
                mus_vol: 0,
            });
        };

        let sfx: Vec<SfxSample> = SFX_INFO_BASE
            .iter()
            .map(|s| {
                let name = format!("DS{}", s.name.to_ascii_uppercase());
                let decoded = wad.get_lump(&name).and_then(|l| decode_sfx_lump(&l.data));
                if decoded.is_none() {
                    debug!("{name} is missing");
                }
                let (rate, data) = decoded.unwrap_or((11_025, Vec::new()));
                SfxSample {
                    rate,
                    data,
                    priority: s.priority,
                }
            })
            .collect();
        info!("Initialised {} sfx", sfx.len());

        let soundfont =
            setup
                .soundfont
                .and_then(|bytes| match SoundFont::new(&mut Cursor::new(&bytes)) {
                    Ok(font) => Some(Arc::new(font)),
                    Err(e) => {
                        warn!("Failed to parse SoundFont: {e:?}");
                        None
                    }
                });

        if soundfont.is_some() {
            let mut mus_count = 0;
            let mut missing = 0;
            // SAFETY: single-threaded startup code; the sound-server thread
            // that reads MUS_DATA hasn't started yet.
            #[allow(static_mut_refs)]
            unsafe {
                for mus in MUS_DATA.iter_mut() {
                    let name = mus.lump_name();
                    if let Some(lump) = wad.get_lump(name.as_str()) {
                        if lump.data.len() < 4 {
                            warn!("{name} is too short");
                        } else if lump.data[..4] == MUS_ID {
                            if let Some(res) = read_mus_to_midi(&lump.data) {
                                mus.set_data(res);
                                mus_count += 1;
                            } else {
                                warn!("{name}: MUS to MIDI conversion failed");
                            }
                        } else if lump.data[..4] == MID_ID {
                            mus.set_data(lump.data.clone());
                            mus_count += 1;
                        } else {
                            warn!("{name}: unrecognized music format {:02x?}", &lump.data[..4]);
                        }
                    } else {
                        missing += 1;
                        debug!("{name} is missing");
                    }
                }
            }
            info!("Initialised {mus_count} midi songs ({missing} lumps not in WAD)");
        } else {
            info!("No SoundFont available: music disabled");
        }

        let mixer = Arc::new(Mutex::new(Mixer::new(sfx)));
        let mixer_thread = {
            let mixer = mixer.clone();
            let running = running.clone();
            std::thread::Builder::new()
                .name("sound-mixer".to_string())
                .stack_size(64 * 1024)
                .spawn(move || run_mixer(mixer, setup.sink, running))?
        };

        Ok(Self {
            rx,
            tx,
            mixer: Some(mixer),
            mixer_thread: Some(mixer_thread),
            soundfont,
            running,
            sfx_vol: MAX_VOLUME,
            mus_vol: MAX_VOLUME,
        })
    }

    fn with_mixer(&mut self, f: impl FnOnce(&mut Mixer)) {
        if let Some(mixer) = &self.mixer {
            f(&mut mixer.lock().unwrap());
        }
    }
}

impl Drop for Snd {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.mixer_thread.take() {
            let _ = handle.join();
        }
    }
}

impl SoundServer<SfxName, usize, SndError> for Snd {
    fn init(&mut self) -> InitResult<SfxName, usize, SndError> {
        Ok(self.tx.clone())
    }

    fn start_sound(&mut self, uid: usize, sfx: SfxName, x: f32, y: f32) {
        self.with_mixer(|m| m.start_sound(uid, sfx, x, y));
    }

    fn update_listener(&mut self, uid: usize, x: f32, y: f32, angle: f32) {
        self.with_mixer(|m| m.update_listener(uid, x, y, angle));
    }

    fn stop_sound(&mut self, uid: usize) {
        self.with_mixer(|m| m.stop_sound(uid));
    }

    fn stop_sound_all(&mut self) {
        self.with_mixer(|m| m.stop_sound_all());
    }

    fn set_sfx_volume(&mut self, volume: i32) {
        self.sfx_vol = volume.clamp(0, MAX_VOLUME);
        let gain = volume_to_gain(volume);
        self.with_mixer(|m| m.sfx_gain = gain);
    }

    fn get_sfx_volume(&mut self) -> i32 {
        self.sfx_vol
    }

    fn start_music(&mut self, music: usize, looping: bool) {
        // Stop the current track first; the replacement may fail to build.
        self.with_mixer(|m| {
            m.music = None;
            m.music_paused = false;
        });
        let Some(font) = self.soundfont.clone() else {
            return;
        };
        // SAFETY: MUS_DATA is only written during `Snd::new`, before the
        // sound-server thread starts processing commands.
        #[allow(static_mut_refs)]
        let data = unsafe { MUS_DATA[music].data() };
        if data.is_empty() {
            debug!("music: no data for track {music}");
            return;
        }
        let midi = match MidiFile::new(&mut Cursor::new(data)) {
            Ok(m) => Arc::new(m),
            Err(e) => {
                warn!("music: failed to parse track {music}: {e:?}");
                return;
            }
        };
        // Building the synthesizer is expensive; do it before taking the
        // mixer lock so audio keeps flowing during track changes.
        let mut settings = SynthesizerSettings::new(OUTPUT_RATE as i32);
        settings.maximum_polyphony = MUSIC_POLYPHONY;
        settings.enable_reverb_and_chorus = false;
        let mut synthesizer = match Synthesizer::new(&font, &settings) {
            Ok(s) => s,
            Err(e) => {
                warn!("music: failed to create synthesizer: {e:?}");
                return;
            }
        };
        synthesizer.set_master_volume(MUSIC_MASTER_VOLUME);
        let mut sequencer = MidiFileSequencer::new(synthesizer);
        sequencer.play(&midi, looping);
        self.with_mixer(|m| m.music = Some(sequencer));
    }

    fn pause_music(&mut self) {
        self.with_mixer(|m| m.music_paused = true);
    }

    fn resume_music(&mut self) {
        self.with_mixer(|m| m.music_paused = false);
    }

    fn change_music(&mut self, music: usize, looping: bool) {
        self.start_music(music, looping);
    }

    fn stop_music(&mut self) {
        self.with_mixer(|m| m.music = None);
    }

    fn set_mus_volume(&mut self, volume: i32) {
        self.mus_vol = volume.clamp(0, MAX_VOLUME);
        let gain = volume_to_gain(volume);
        self.with_mixer(|m| m.mus_gain = gain);
    }

    fn get_mus_volume(&mut self) -> i32 {
        self.mus_vol
    }

    fn update_self(&mut self) {}

    fn get_rx(&mut self) -> &mut SndServerRx {
        &mut self.rx
    }

    fn shutdown_sound(&mut self) {
        info!("Shutdown sound server");
        self.stop_sound_all();
        self.stop_music();
        self.running.store(false, Ordering::Relaxed);
    }
}

impl SoundServerTic<SfxName, usize, SndError> for Snd {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_shareware_music() {
        let wad = WadData::new("../doom1.wad".into());
        let mut converted = 0;
        #[allow(static_mut_refs)]
        unsafe {
            for mus in MUS_DATA.iter_mut() {
                if let Some(lump) = wad.get_lump(mus.lump_name().as_str()) {
                    if lump.data.len() >= 4 && lump.data[..4] == MUS_ID {
                        if let Some(res) = read_mus_to_midi(&lump.data) {
                            mus.set_data(res);
                            converted += 1;
                        }
                    }
                } else {
                    println!("{} is missing", mus.lump_name());
                }
            }
        }
        println!("converted {converted} tracks");
        assert!(converted > 0);
    }

    #[test]
    fn decode_shareware_sfx() {
        let wad = WadData::new("../doom1.wad".into());
        let decoded = SFX_INFO_BASE
            .iter()
            .filter_map(|s| wad.get_lump(&format!("DS{}", s.name.to_ascii_uppercase())))
            .filter_map(|l| decode_sfx_lump(&l.data))
            .count();
        println!("decoded {decoded} sfx");
        assert!(decoded >= 50);
    }
}
