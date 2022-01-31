use std::collections::{hash_map, HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;
use tokio::time::timeout;

use crate::psmove::Controller;

pub type ControllerId = u64;

pub struct Controllers {
    controllers: HashMap<ControllerId, Controller>,
}

impl Controllers {
    const TIMEOUT: Duration = Duration::from_millis(10);

    pub async fn init() -> Result<Self> {
        return Ok(Self {
            controllers: HashMap::new(),
        });
    }

    pub fn register(&mut self, controller: Controller) {
        self.controllers.insert(controller.id(), controller);
    }

    pub async fn reinit(&mut self) -> Result<()> {
        return Ok(());
    }

    pub async fn update(&mut self) -> Result<()> {
        let updates = self.iter_mut()
            .map(|controller| timeout(Self::TIMEOUT, controller.update()));

        futures::future::join_all(updates).await;

        return Ok(());
    }

    pub fn count(&self) -> usize {
        return self.controllers.len();
    }

    pub fn iter(&self) -> impl Iterator<Item=&Controller> + ExactSizeIterator {
        return self.controllers.values();
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut Controller> + ExactSizeIterator {
        return self.controllers.values_mut();
    }

    pub fn with_data<'a, D>(&'a mut self, data: &'a mut PlayerData<D>) -> WithData<'a, D> {
        return WithData {
            iter: self.controllers.values_mut(),
            data,
        };
    }
}

pub struct PlayerData<D> {
    data: HashMap<ControllerId, D>,
}

impl<D> PlayerData<D> {
    pub fn init(players: HashSet<ControllerId>, f: impl Fn() -> D) -> Self {
        let data = players.into_iter()
            .map(|id| (id, f()))
            .collect();

        return Self { data };
    }

    pub fn init_with(data: HashMap<ControllerId, D>) -> Self {
        return Self { data };
    }

    pub fn new() -> Self {
        return Self { data: HashMap::new() };
    }

    pub fn reset(&mut self) {
        self.data.clear();
    }

    pub fn get(&mut self, controller: &Controller) -> Option<&mut D> {
        return self.data.get_mut(&controller.id());
    }

    pub fn iter(&self) -> impl Iterator<Item=(ControllerId, &D)> {
        return self.data.iter()
            .map(|(id, data)| (*id, data));
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=(ControllerId, &mut D)> {
        return self.data.iter_mut()
            .map(|(id, data)| (*id, data));
    }
}

pub struct WithData<'a, D> {
    iter: hash_map::ValuesMut<'a, ControllerId, Controller>,
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

    pub fn existing(self) -> impl Iterator<Item=(&'a mut Controller, &'a mut D)> {
        return self.filter_map(|(controller, data)| data.map(|data| (controller, data)));
    }
}

impl<'a, D> Iterator for WithData<'a, D> {
    type Item = (&'a mut Controller, Option<&'a mut D>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(controller) = self.iter.next() {
            let data = self.data.get(controller);
            // SAFETY: This is save because the underlying `self.iter` is guaranteed to yield unique
            // serials and therefore this will never hand out two references to the same element
            // from `self.data`.
            let data: Option<&mut D> = unsafe { std::mem::transmute(data) };
            return Some((controller, data));
        };

        return None;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return self.iter.size_hint();
    }
}

pub struct WithDefaultData<'a, D, F> {
    iter: hash_map::ValuesMut<'a, ControllerId, Controller>,
    data: &'a mut PlayerData<D>,
    default: F,
}

impl<'a, D, F> Iterator for WithDefaultData<'a, D, F>
    where
        F: Fn() -> D,
{
    type Item = (&'a mut Controller, &'a mut D);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(controller) = self.iter.next() {
            let data = self.data.data.entry(controller.id())
                .or_insert_with(|| (self.default)());

            // SAFETY: This is save because the underlying `self.iter` is guaranteed to yield unique
            // serials and therefore this will never hand out two references to the same element
            // from `self.data`.
            let data: &mut D = unsafe { std::mem::transmute(data) };

            return Some((controller, data));
        };

        return None;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return self.iter.size_hint();
    }
}
