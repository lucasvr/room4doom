use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
        Arc,
    },
    time::Duration,
};

mod sounds;
pub use sounds::*;

/// `S` is SFX enum, `M` is Music enum, `E` is Errors
pub type InitResult<S, M, E> = Result<(Sender<SoundAction<S, M>>, Arc<AtomicBool>), E>;

// Need sound_origin, player_origin, player_angle...
// should be trait to get basic positioning.

#[derive(Debug, Default, Clone, Copy)]
pub struct SoundObjPosition {
    /// Objects unique ID or hash. This should be used to track which
    /// object owns which sounds so it can be stopped e.g, death, shoot..
    uid: usize,
    /// The world XY coords of this object
    pos: (f32, f32),
    /// Get the angle of this object in radians
    angle: f32,
    /// Channel allocated to it (internal)
    pub channel: i32,
}

impl SoundObjPosition {
    /// The UID is used to track which playing sound is owned by which object.
    pub fn new(uid: usize, pos: (f32, f32), angle: f32) -> Self {
        Self {
            uid,
            pos,
            angle,
            channel: 0,
        }
    }

    pub fn x(&self) -> f32 {
        self.pos.0
    }

    pub fn y(&self) -> f32 {
        self.pos.1
    }

    pub fn pos(&self) -> (f32, f32) {
        self.pos
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn uid(&self) -> usize {
        self.uid
    }
}

pub enum SoundAction<S: Debug, M: Debug> {
    StartSfx {
        origin: SoundObjPosition,
        sfx: S,
    },
    UpdateSound {
        listener: SoundObjPosition,
    },
    StopSfx {
        uid: usize,
    },
    SfxVolume(i32),
    MusicVolume(i32),

    /// Music ID and looping/not
    StartMusic(M, bool),
    PauseMusic,
    ResumeMusic,
    ChangeMusic(M, bool),
    StopMusic,
}

/// A sound server implementing `SoundServer` must also implement `SoundServerTic`
/// typically by a one-liner: `impl SoundServerTic<SndFx> for Snd {}`
pub trait SoundServer<S, M, E>
where
    S: Debug,
    M: Debug,
    E: std::error::Error,
{
    /// Start up all sound stuff and grab the `Sender` channel for cloning, and an
    /// `AtomicBool` to stop sound and deinitialise devices etc in preparation for
    /// game exit.
    fn init(&mut self) -> InitResult<S, M, E>;

    /// Playback a sound
    fn start_sound(&mut self, origin: SoundObjPosition, sound: S);

    /// Update a sounds parameters
    fn update_sound(&mut self, listener: SoundObjPosition);

    /// Stop this sound playback
    fn stop_sound(&mut self, uid: usize);

    fn set_sfx_volume(&mut self, volume: i32);

    fn get_sfx_volume(&mut self) -> i32;

    fn start_music(&mut self, music: M, looping: bool);

    fn pause_music(&mut self);

    fn resume_music(&mut self);

    fn change_music(&mut self, music: M, looping: bool);

    fn stop_music(&mut self);

    fn set_mus_volume(&mut self, volume: i32);

    fn get_mus_volume(&mut self) -> i32;

    /// Start, stop, change, remove sounds. Anythign that a sound server needs
    /// to do each tic
    fn update_self(&mut self);

    /// Helper function used by the `SoundServerTic` trait
    fn get_rx(&mut self) -> &mut Receiver<SoundAction<S, M>>;

    /// Atomic for shutting down the `SoundServer`
    fn get_shutdown(&self) -> &AtomicBool;

    /// Stop all sound and release the sound device
    fn shutdown_sound(&mut self);
}

/// Run the `SoundServer`
pub trait SoundServerTic<S, M, E>
where
    Self: SoundServer<S, M, E>,
    S: Debug,
    M: Debug,
    E: std::error::Error,
{
    /// Will be called every period on a thread containing `SoundServer`
    fn tic(&mut self) {
        if let Ok(sound) = self.get_rx().recv_timeout(Duration::from_micros(500)) {
            match sound {
                SoundAction::StartSfx { origin, sfx } => self.start_sound(origin, sfx),
                SoundAction::UpdateSound { listener } => self.update_sound(listener),
                SoundAction::StopSfx { uid } => self.stop_sound(uid),
                SoundAction::StartMusic(music, looping) => self.start_music(music, looping),
                SoundAction::PauseMusic => self.pause_music(),
                SoundAction::ResumeMusic => self.resume_music(),
                SoundAction::ChangeMusic(music, looping) => self.change_music(music, looping),
                SoundAction::StopMusic => self.stop_music(),
                SoundAction::SfxVolume(v) => self.set_sfx_volume(v),
                SoundAction::MusicVolume(v) => self.set_mus_volume(v),
            }
        }

        if self.get_shutdown().load(Ordering::SeqCst) {
            self.get_shutdown().store(false, Ordering::Relaxed);
            self.shutdown_sound();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        f32::consts::PI,
        fmt::Display,
        sync::{
            atomic::AtomicBool,
            mpsc::{channel, Receiver, Sender},
            Arc,
        },
    };

    use crate::{InitResult, SoundAction, SoundObjPosition, SoundServer, SoundServerTic};

    #[derive(Debug)]
    enum FxError {}

    impl Error for FxError {}

    impl Display for FxError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&format!("{:?}", self))
        }
    }

    #[derive(Debug)]
    enum SndFx {
        One,
    }

    #[derive(Debug)]
    enum Music {
        E1M1,
    }

    struct Snd {
        rx: Receiver<SoundAction<SndFx, Music>>,
        tx: Sender<SoundAction<SndFx, Music>>,
        kill: Arc<AtomicBool>,
    }

    impl Snd {
        fn new() -> Self {
            let (tx, rx) = channel();
            Self {
                rx,
                tx,
                kill: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl SoundServer<SndFx, Music, FxError> for Snd {
        fn init(&mut self) -> InitResult<SndFx, Music, FxError> {
            Ok((self.tx.clone(), self.kill.clone()))
        }

        fn start_sound(&mut self, origin: SoundObjPosition, sound: SndFx) {
            dbg!(sound);
        }

        fn update_sound(&mut self, listener: SoundObjPosition) {
            dbg!(listener);
        }

        fn stop_sound(&mut self, uid: usize) {
            dbg!(uid);
        }

        fn start_music(&mut self, music: Music, _looping: bool) {
            dbg!(music);
        }

        fn pause_music(&mut self) {}

        fn resume_music(&mut self) {}

        fn change_music(&mut self, _music: Music, _looping: bool) {}

        fn stop_music(&mut self) {}

        fn set_sfx_volume(&mut self, volume: i32) {}

        fn get_sfx_volume(&mut self) -> i32 {
            6
        }

        fn set_mus_volume(&mut self, volume: i32) {}

        fn get_mus_volume(&mut self) -> i32 {
            7
        }

        fn update_self(&mut self) {}

        fn get_rx(&mut self) -> &mut Receiver<SoundAction<SndFx, Music>> {
            &mut self.rx
        }

        fn get_shutdown(&self) -> &AtomicBool {
            self.kill.as_ref()
        }

        fn shutdown_sound(&mut self) {
            todo!()
        }
    }

    impl SoundServerTic<SndFx, Music, FxError> for Snd {}

    #[test]
    fn run_tic() {
        let mut snd = Snd::new();
        let (tx, _kill) = snd.init().unwrap();

        tx.send(SoundAction::StartSfx {
            origin: SoundObjPosition::new(123, (0.3, 0.3), PI),
            sfx: SndFx::One,
        })
        .unwrap();
        tx.send(SoundAction::UpdateSound {
            listener: SoundObjPosition::new(123, (1.0, 1.0), PI / 2.0),
        })
        .unwrap();
        tx.send(SoundAction::StopSfx { uid: 123 }).unwrap();
        assert_eq!(snd.rx.try_iter().count(), 3);

        tx.send(SoundAction::StartSfx {
            origin: SoundObjPosition::new(123, (0.3, 0.3), PI),
            sfx: SndFx::One,
        })
        .unwrap();
        tx.send(SoundAction::UpdateSound {
            listener: SoundObjPosition::new(123, (1.0, 1.0), PI / 2.0),
        })
        .unwrap();
        tx.send(SoundAction::StopSfx { uid: 123 }).unwrap();
        for _ in 0..3 {
            snd.tic();
        }

        assert_eq!(snd.rx.try_iter().count(), 0);
    }
}
