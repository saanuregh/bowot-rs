use rand::seq::SliceRandom;
use rand::thread_rng;
use serenity::{
    model::id::{GuildId, UserId},
    voice::LockedAudio,
};

#[derive(Clone)]
pub struct Track {
    pub url: String,
    pub title: String,
    pub requester: UserId,
    pub live: bool,
}

#[derive(Clone)]
pub struct Player {
    guild_id: GuildId,
    pub queue: Vec<Track>,
    pub now_source: Option<LockedAudio>,
    pub repeat: Repeat,
}

#[derive(Clone)]
pub enum Repeat {
    Off,
    One,
    All,
}

impl Player {
    pub fn new(guild_id: GuildId) -> Self {
        Player {
            guild_id: guild_id,
            queue: Vec::new(),
            now_source: None,
            repeat: Repeat::Off,
        }
    }

    pub fn is_empty(self) -> bool {
        self.queue.is_empty()
    }

    pub fn add_track(&mut self, track: Track) -> &mut Self {
        self.queue.push(track);
        self
    }

    pub fn remove_track(&mut self, index: usize) -> Option<Track> {
        if self.queue.get(index).is_some() {
            Some(self.queue.remove(index))
        } else {
            None
        }
    }

    pub fn push(&mut self, track: Track) -> &mut Self {
        self.queue.push(track);
        self
    }

    pub fn pop(&mut self) -> Option<Track> {
        if !self.queue.is_empty() {
            Some(self.queue.remove(0))
        } else {
            None
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.queue = Vec::new();
        self
    }

    pub fn clear_except_np(&mut self) -> &mut Self {
        self.queue = vec![self.queue[0].clone()];
        self
    }

    pub fn shuffle(&mut self) -> &mut Self {
        let np = self.queue[0].clone();
        self.queue = self.queue[1..].to_vec();
        self.queue.shuffle(&mut thread_rng());
        self.queue.insert(0, np);
        self
    }

    pub fn set_now_source(&mut self, source: LockedAudio) -> &mut Self {
        self.now_source = Some(source);
        self
    }

    pub fn set_repeat(&mut self, repeat: Repeat) -> &mut Self {
        self.repeat = repeat;
        self
    }

    pub fn reset(&mut self) -> &mut Self {
        self.now_source = None;
        self
    }

    pub async fn is_finished(&mut self) -> bool {
        match self.now_source.clone() {
            Some(audio_lock) => {
                let audio = audio_lock.lock().await;
                if audio.finished {
                    true
                } else {
                    false
                }
            }
            None => true,
        }
    }

    pub async fn is_paused(&mut self) -> bool {
        match self.now_source.clone() {
            Some(audio_lock) => {
                let audio = audio_lock.lock().await;
                if !audio.playing {
                    true
                } else {
                    false
                }
            }
            None => true,
        }
    }

    pub async fn pause(&mut self) -> &mut Self {
        if let Some(audio_lock) = self.now_source.clone() {
            let mut audio = audio_lock.lock().await;
            if !audio.finished {
                audio.pause();
            }
        }
        self
    }

    pub async fn play(&mut self) -> &mut Self {
        if let Some(audio_lock) = self.now_source.clone() {
            let mut audio = audio_lock.lock().await;
            if !audio.finished {
                audio.play();
            }
        }
        self
    }
}
