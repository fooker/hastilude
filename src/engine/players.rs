use std::collections::{hash_map, HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;
use scarlet::color::RGBColor;
use tokio::time::timeout;

use crate::engine::animation::Animated;
use crate::psmove::{Battery, Controller, Feedback, Input};

pub type PlayerId = u64;

pub struct Players {
    players: HashMap<PlayerId, Player>,
}

pub struct Player {
    controller: Controller,

    pub rumble: Animated<u8>,
    pub color: Animated<RGBColor>,
}

impl Player {
    pub fn id(&self) -> PlayerId {
        return self.controller.id();
    }

    pub fn input(&self) -> &Input {
        return self.controller.input();
    }

    pub fn battery(&self) -> Battery {
        return self.controller.battery();
    }

    async fn update(&mut self, duration: Duration) -> Result<()> {
        self.rumble.update(duration);

        self.controller.feedback(Feedback {
            rgb: (0, 0, 0),
            rumble: self.rumble.value(),
        });

        return self.controller.update().await;
    }
}

impl Players {
    const TIMEOUT: Duration = Duration::from_millis(10);

    pub async fn init() -> Result<Self> {
        return Ok(Self {
            players: HashMap::new(),
        });
    }

    pub fn register(&mut self, controller: Controller) {
        self.players.insert(controller.id(), Player {
            controller,
            rumble: Animated::idle(0),
            color: Animated::idle(RGBColor { r: 0.0, g: 0.0, b: 0.0 }),
        });
    }

    pub async fn reinit(&mut self) -> Result<()> {
        return Ok(());
    }

    pub async fn update(&mut self, duration: Duration) -> Result<()> {
        // TODO: Check and handle (log, retry, circuit-break) timeouts

        let updates = self.iter_mut()
            .map(|player| timeout(Self::TIMEOUT, player.update(duration)));

        futures::future::join_all(updates).await;

        return Ok(());
    }

    pub fn count(&self) -> usize {
        return self.players.len();
    }

    pub fn iter(&self) -> impl Iterator<Item=&Player> + ExactSizeIterator {
        return self.players.values();
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut Player> + ExactSizeIterator {
        return self.players.values_mut();
    }

    pub fn with_data<'a, D>(&'a mut self, data: &'a mut PlayerData<D>) -> WithData<'a, D> {
        return WithData {
            iter: self.players.values_mut(),
            data,
        };
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

    pub fn get(&mut self, player: PlayerId) -> Option<&mut D> {
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
}

pub struct WithData<'a, D> {
    iter: hash_map::ValuesMut<'a, PlayerId, Player>,
    data: &'a mut PlayerData<D>,
}

impl<'a, D> WithData<'a, D> {
    pub fn with_default<F>(self, default: F) -> WithDefaultData<'a, D, F> {
        return WithDefaultData {
            iter: self.iter,
            data: self.data,
            default,
        };
    }

    pub fn existing(self) -> impl Iterator<Item=(&'a mut Player, &'a mut D)> {
        return self.filter_map(|(player, data)| data.map(|data| (player, data)));
    }
}

impl<'a, D> Iterator for WithData<'a, D> {
    type Item = (&'a mut Player, Option<&'a mut D>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(player) = self.iter.next() {
            let data = self.data.get(player.id());
            // SAFETY: This is save because the underlying `self.iter` is guaranteed to yield unique
            // serials and therefore this will never hand out two references to the same element
            // from `self.data`.
            let data: Option<&mut D> = unsafe { std::mem::transmute(data) };
            return Some((player, data));
        };

        return None;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return self.iter.size_hint();
    }
}

pub struct WithDefaultData<'a, D, F> {
    iter: hash_map::ValuesMut<'a, PlayerId, Player>,
    data: &'a mut PlayerData<D>,
    default: F,
}

impl<'a, D, F> Iterator for WithDefaultData<'a, D, F>
    where
        F: Fn() -> D,
{
    type Item = (&'a mut Player, &'a mut D);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(player) = self.iter.next() {
            let data = self.data.data.entry(player.id())
                .or_insert_with(|| (self.default)());

            // SAFETY: This is save because the underlying `self.iter` is guaranteed to yield unique
            // serials and therefore this will never hand out two references to the same element
            // from `self.data`.
            let data: &mut D = unsafe { std::mem::transmute(data) };

            return Some((player, data));
        };

        return None;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return self.iter.size_hint();
    }
}
