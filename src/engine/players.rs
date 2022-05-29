use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;
use cgmath::InnerSpace;
use futures::{StreamExt, task::Poll};
use heapless::HistoryBuffer;
use scarlet::color::RGBColor;
use tokio::time::timeout;
use tracing::{debug, error, instrument, warn};

use crate::controller::{Battery, Controller, Feedback, hid, Input};
use crate::engine::animation::Animated;

pub type PlayerId = u64;

pub struct Player {
    controller: Controller,

    acceleration: HistoryBuffer<f32, 4>,

    pub rumble: Animated<u8>,
    pub color: Animated<RGBColor>,

    failed: usize,
}

impl Player {
    const TIMEOUT: Duration = Duration::from_millis(1000);

    pub fn id(&self) -> PlayerId {
        return self.controller.id();
    }

    pub fn input(&self) -> &Input {
        return self.controller.input();
    }

    pub fn battery(&self) -> Battery {
        return self.controller.battery();
    }

    #[instrument(level = "trace", name = "Player::update", skip(self), fields(id = self.id()))]
    async fn update(&mut self, duration: Duration) {
        self.rumble.update(duration);
        self.color.update(duration);

        self.controller.feedback(Feedback {
            rgb: self.color.value().int_rgb_tup(),
            rumble: self.rumble.value(),
        });

        let update = self.controller.update();
        let update = timeout(Self::TIMEOUT, update);

        if let Err(err) = update.await
            .map_err(Into::into)
            .flatten() {
            warn!("Updating controller {} failed: {}", self.controller.id(), err);
            self.failed += 1;
        } else {
            // TODO: Do not reset immediately but require multiple successful before resetting
            // TODO: Report flaky devices
            self.failed = 0;
        }

        // Update acceleration data history
        self.acceleration.write((1.0 - self.controller.input().accelerometer.magnitude()).abs());
    }

    pub fn controller(&self) -> &Controller {
        return &self.controller;
    }

    pub fn acceleration(&self, avg: bool) -> f32 {
        return if avg {
            self.acceleration.iter().sum::<f32>() / self.acceleration.len() as f32
        } else {
            self.acceleration.recent().copied().unwrap_or(0.0)
        };
    }
}

pub struct Players {
    players: Vec<Player>,

    events: hid::Events,
}

impl Players {
    const MAX_FAILS: usize = 10;

    #[instrument(level = "debug")]
    pub async fn init() -> Result<Self> {
        let (devices, events) = hid::monitor()?;

        let mut players = Self {
            players: Vec::new(),
            events,
        };

        // Process all initial devices
        for device in devices {
            players.add_device(device).await?;
        }

        return Ok(players);
    }

    #[instrument(level = "trace", name = "Players::update", skip(self))]
    pub async fn update(&mut self, duration: Duration) -> Result<()> {
        // We limit this to a single event on each update cycle
        if let Poll::Ready(Some(event)) = futures::poll(self.events.next()).await {
            match event? {
                hid::Event::Added(device) => {
                    self.add_device(device).await?;
                }

                hid::Event::Removed(path) => {
                    debug!("Removed controller: {:?}", &path);
                    self.players.retain(|player| player.controller.path() != path);
                }
            };
        }

        // Update all controllers
        futures::future::join_all(
            self.players.iter_mut()
                .map(|player| player.update(duration))
        ).await;

        // Drop controllers with high error count
        for player in self.players
            .drain_filter(|player| player.failed >= Self::MAX_FAILS) {
            error!("Dropping player {} because of to many errors", player.id());
        }

        return Ok(());
    }

    pub fn count(&self) -> usize {
        return self.players.len();
    }

    pub fn iter(&self) -> impl Iterator<Item=&Player> + ExactSizeIterator {
        return self.players.iter();
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut Player> + ExactSizeIterator {
        return self.players.iter_mut();
    }

    pub fn get(&self, id: PlayerId) -> Option<&Player> {
        return self.players.iter().find(|player| player.id() == id);
    }

    pub fn get_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        return self.players.iter_mut().find(|player| player.id() == id);
    }

    pub fn keys(&self) -> impl Iterator<Item=PlayerId> + '_ {
        return self.players.iter().map(Player::id);
    }

    pub fn with_data<'a, D>(&'a mut self, data: &'a mut PlayerData<D>) -> WithData<'a, D> {
        return WithData {
            players: self,
            data,
        };
    }

    async fn add_device(&mut self, device: hid::Device) -> Result<()> {
        debug!("Added controller: {:?}", device.path);

        let controller = Controller::new(&device.path).await?;

        // Must ensure IDs are unique
        assert!(self.players.iter()
            .map(Player::id)
            .find(|id| *id == controller.id())
            .is_none());

        self.players.push(Player {
            controller,
            acceleration: HistoryBuffer::new_with(0.0),
            rumble: Animated::idle(0),
            color: Animated::idle(RGBColor { r: 0.0, g: 0.0, b: 0.0 }),
            failed: 0,
        });

        return Ok(());
    }
}

pub struct PlayerData<D> {
    data: HashMap<PlayerId, D>,
}

impl<D> PlayerData<D> {
    pub fn init(players: HashSet<PlayerId>, f: impl Fn() -> D) -> Self {
        let data = players.into_iter()
            .map(|id| (id, f()))
            .collect();

        return Self { data };
    }

    pub fn init_with(data: HashMap<PlayerId, D>) -> Self {
        return Self { data };
    }

    pub fn new() -> Self {
        return Self { data: HashMap::new() };
    }

    pub fn reset(&mut self) {
        self.data.clear();
    }

    pub fn get(&mut self, player: PlayerId) -> Option<&D> {
        return self.data.get(&player);
    }

    pub fn get_mut(&mut self, player: PlayerId) -> Option<&mut D> {
        return self.data.get_mut(&player);
    }

    pub fn iter(&self) -> impl Iterator<Item=(PlayerId, &D)> {
        return self.data.iter()
            .map(|(id, data)| (*id, data));
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=(PlayerId, &mut D)> {
        return self.data.iter_mut()
            .map(|(id, data)| (*id, data));
    }

    pub fn keys(&self) -> impl Iterator<Item=PlayerId> + '_ {
        return self.data.keys().copied();
    }

    pub fn remove(&mut self, player: PlayerId) -> bool {
        return self.data.remove(&player).is_some();
    }

    pub fn len(&self) -> usize {
        return self.data.len();
    }
}

pub struct WithData<'a, D> {
    players: &'a mut Players,
    data: &'a mut PlayerData<D>,
}

impl<'a, D> WithData<'a, D> {
    pub fn update(self, mut f: impl FnMut(&'a mut Player, &'a mut D) -> bool) {
        self.data.data.retain(|id, data| {
            if let Some(player) = self.players.get_mut(*id) {
                // SAFETY: This is save because the underlying `self.iter` is guaranteed to yield unique
                // serials and therefore this will never hand out two references to the same element
                // from `self.data`.
                let data: &'a mut D = unsafe { std::mem::transmute(data) };
                let player: &'a mut Player = unsafe { std::mem::transmute(player) };
                return f(player, data);
            }

            return false;
        })
    }
}
