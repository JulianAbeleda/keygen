//! Deterministic audio ownership and scheduling state.
//!
//! This module deliberately stops at logical PCM playback state. Host adapters
//! (CoreAudio on macOS) consume [`AudioCommand`] and report no state back into
//! the deterministic reducer.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum AudioChannel {
    Music,
    PoemMusic,
    Sound,
    Launcher,
    Jukebox,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AudioOwner {
    Launcher,
    Story,
    Jukebox,
    System,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AudioClip {
    pub id: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub loop_start: Option<u64>,
    pub loop_end: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChannelState {
    pub owner: AudioOwner,
    pub current: Option<AudioClip>,
    pub queued: Vec<AudioClip>,
    pub position: u64,
    pub volume: f32,
    pub fade: Option<Fade>,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fade {
    pub from: f32,
    pub to: f32,
    pub duration_frames: u64,
    pub elapsed_frames: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AudioCommand {
    Play {
        channel: AudioChannel,
        clip: AudioClip,
        owner: AudioOwner,
        looped: bool,
    },
    Stop {
        channel: AudioChannel,
    },
    Queue {
        channel: AudioChannel,
        clip: AudioClip,
    },
    Fade {
        channel: AudioChannel,
        to: f32,
        duration_frames: u64,
    },
    Cancel {
        channel: AudioChannel,
        generation: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioSnapshot {
    pub channels: Vec<(AudioChannel, ChannelState)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioGraph {
    channels: std::collections::BTreeMap<AudioChannel, ChannelState>,
}

impl Default for AudioGraph {
    fn default() -> Self {
        Self::new(AudioOwner::System)
    }
}

impl AudioGraph {
    pub fn new(default_owner: AudioOwner) -> Self {
        let channels = [
            AudioChannel::Music,
            AudioChannel::PoemMusic,
            AudioChannel::Sound,
            AudioChannel::Launcher,
            AudioChannel::Jukebox,
        ]
        .into_iter()
        .map(|channel| {
            (
                channel,
                ChannelState {
                    owner: default_owner,
                    current: None,
                    queued: Vec::new(),
                    position: 0,
                    volume: 1.0,
                    fade: None,
                    generation: 0,
                },
            )
        })
        .collect();
        Self { channels }
    }

    pub fn channel(&self, channel: AudioChannel) -> &ChannelState {
        &self.channels[&channel]
    }
    pub fn channel_mut(&mut self, channel: AudioChannel) -> &mut ChannelState {
        self.channels.get_mut(&channel).expect("all channels exist")
    }

    pub fn play(
        &mut self,
        channel: AudioChannel,
        clip: AudioClip,
        owner: AudioOwner,
        looped: bool,
    ) -> u64 {
        let state = self.channel_mut(channel);
        state.owner = owner;
        state.current = Some(clip.clone());
        state.queued.clear();
        state.position = 0;
        state.fade = None;
        state.generation = state.generation.wrapping_add(1);
        if looped {
            state.queued.push(clip);
        }
        state.generation
    }

    pub fn stop(&mut self, channel: AudioChannel) {
        let state = self.channel_mut(channel);
        state.current = None;
        state.queued.clear();
        state.position = 0;
        state.fade = None;
        state.generation = state.generation.wrapping_add(1);
    }
    pub fn queue(&mut self, channel: AudioChannel, clip: AudioClip) {
        self.channel_mut(channel).queued.push(clip);
    }

    pub fn fade(&mut self, channel: AudioChannel, to: f32, duration_frames: u64) {
        let state = self.channel_mut(channel);
        state.fade = Some(Fade {
            from: state.volume,
            to: to.clamp(0.0, 1.0),
            duration_frames,
            elapsed_frames: 0,
        });
        if duration_frames == 0 {
            state.volume = to.clamp(0.0, 1.0);
            state.fade = None;
        }
    }

    pub fn tick(&mut self, frames: u64) {
        for state in self.channels.values_mut() {
            if let Some(fade) = &mut state.fade {
                fade.elapsed_frames = fade
                    .elapsed_frames
                    .saturating_add(frames)
                    .min(fade.duration_frames);
                let ratio = fade.elapsed_frames as f32 / fade.duration_frames as f32;
                state.volume = fade.from + (fade.to - fade.from) * ratio;
                if fade.elapsed_frames == fade.duration_frames {
                    state.fade = None;
                }
            }
            if let Some(clip) = &state.current {
                state.position = state.position.saturating_add(frames);
                if state.position >= clip.frames {
                    if let Some(next) = state.queued.first().cloned() {
                        state.current = Some(next);
                        state.queued.remove(0);
                        state.position = 0;
                    } else {
                        state.current = None;
                        state.position = 0;
                    }
                }
            }
        }
    }

    pub fn cancel(&mut self, channel: AudioChannel, generation: u64) -> bool {
        if self.channel(channel).generation == generation {
            self.stop(channel);
            true
        } else {
            false
        }
    }
    pub fn handoff(&mut self, channel: AudioChannel, owner: AudioOwner) {
        self.channel_mut(channel).owner = owner;
    }
    pub fn snapshot(&self) -> AudioSnapshot {
        AudioSnapshot {
            channels: self.channels.iter().map(|(c, s)| (*c, s.clone())).collect(),
        }
    }
    pub fn restore(&mut self, snapshot: AudioSnapshot) {
        self.channels = snapshot.channels.into_iter().collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clip(id: &str, frames: u64) -> AudioClip {
        AudioClip {
            id: id.into(),
            sample_rate: 48_000,
            channels: 2,
            frames,
            loop_start: None,
            loop_end: None,
        }
    }
    #[test]
    fn channels_are_independent_and_owned() {
        let mut graph = AudioGraph::new(AudioOwner::System);
        graph.play(
            AudioChannel::Music,
            clip("m", 10),
            AudioOwner::Launcher,
            false,
        );
        graph.play(AudioChannel::Sound, clip("s", 2), AudioOwner::Story, false);
        assert_eq!(
            graph.channel(AudioChannel::Music).owner,
            AudioOwner::Launcher
        );
        assert_eq!(graph.channel(AudioChannel::Sound).owner, AudioOwner::Story);
        graph.tick(2);
        assert!(graph.channel(AudioChannel::Music).current.is_some());
        assert!(graph.channel(AudioChannel::Sound).current.is_none());
    }
    #[test]
    fn stale_cancellation_cannot_stop_new_generation() {
        let mut graph = AudioGraph::default();
        let old = graph.play(
            AudioChannel::Music,
            clip("a", 5),
            AudioOwner::Launcher,
            false,
        );
        graph.play(AudioChannel::Music, clip("b", 5), AudioOwner::Story, false);
        assert!(!graph.cancel(AudioChannel::Music, old));
        assert_eq!(
            graph
                .channel(AudioChannel::Music)
                .current
                .as_ref()
                .unwrap()
                .id,
            "b"
        );
    }
    #[test]
    fn fade_and_snapshot_are_deterministic() {
        let mut graph = AudioGraph::default();
        graph.play(
            AudioChannel::Music,
            clip("a", 100),
            AudioOwner::Story,
            false,
        );
        graph.fade(AudioChannel::Music, 0.0, 10);
        graph.tick(5);
        let snap = graph.snapshot();
        graph.tick(5);
        graph.restore(snap.clone());
        assert_eq!(graph.snapshot(), snap);
        assert!((graph.channel(AudioChannel::Music).volume - 0.5).abs() < 0.001);
    }
}
