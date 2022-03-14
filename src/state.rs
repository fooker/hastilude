use std::time::Duration;

use thiserror::Error;

use crate::engine::players::{PlayerId, Players};
use crate::engine::World;
use crate::games::Game;
use crate::keyframes;
use crate::meta::celebration::Celebration;
use crate::meta::countdown::Countdown;
use crate::meta::lobby::Lobby;

pub enum State {
    Lobby(Lobby),
    Countdown(Countdown),
    Playing(Box<dyn Game>),
    Celebration(Celebration),
}

impl State {
    pub fn lobby(players: &mut Players) -> Self {
        return Self::Lobby(Lobby::new(players));
    }

    pub fn update(self, world: &mut World, duration: Duration) -> Self {
        return match self {
            State::Lobby(lobby) => lobby.update(world),
            State::Countdown(countdown) => countdown.update(world, duration),
            State::Playing(game) => game.update(world, duration),
            State::Celebration(celebration) => celebration.update(world, duration),
        };
    }

    pub fn start(self, world: &mut World) -> (Self, Result<(), StartGameError>) {
        return match self {
            State::Lobby(lobby) => {
                let (state, ok) = lobby.start(world);
                if ok {
                    (state, Ok(()))
                } else {
                    (state, Err(StartGameError::InsufficientPlayers))
                }
            }

            State::Countdown(_) => (self, Err(StartGameError::AlreadyRunning)),
            State::Playing(_) => (self, Err(StartGameError::AlreadyRunning)),
            State::Celebration(_) => (self, Err(StartGameError::AlreadyRunning)),
        };
    }

    pub fn cancel(self, world: &mut World) -> (Self, Result<(), CancelGameError>) {
        return match self {
            State::Lobby(_) => (self, Err(CancelGameError::GameNotRunning)),
            State::Countdown(_) => (Self::lobby(world.players), Ok(())),
            State::Playing(_) => (Self::lobby(world.players), Ok(())),
            State::Celebration(_) => (self, Err(CancelGameError::GameNotRunning)),
        };
    }

    pub fn buzz_player(self, player: PlayerId, world: &mut World) -> (Self, Result<(), NoSuchPlayerError>) {
        if let Some(player) = world.players.get_mut(player) {
            player.rumble.set_and_animate(0xFF, keyframes![
                1.0 => 0x00 @ end,
            ]);
            return (self, Ok(()));
        } else {
            return (self, Err(NoSuchPlayerError { player }));
        }
    }

    pub fn kick_player(mut self, player: PlayerId, world: &mut World) -> (Self, Result<(), NoSuchPlayerError>) {
        return match self {
            State::Lobby(ref mut lobby) => if lobby.kick_player(player) {
                (self, Ok(()))
            } else {
                (self, Err(NoSuchPlayerError { player }))
            }

            State::Countdown(_) => (self, Err(NoSuchPlayerError { player })),

            State::Playing(ref mut game) => if game.kick_player(player, world) {
                (self, Ok(()))
            } else {
                (self, Err(NoSuchPlayerError { player }))
            }

            State::Celebration(_) => (self, Err(NoSuchPlayerError { player }))
        };
    }
}

#[derive(Error, Debug)]
#[error("No such player: {player}")]
pub struct NoSuchPlayerError {
    player: PlayerId,
}

#[derive(Error, Debug)]
pub enum CancelGameError {
    #[error("Game not running")]
    GameNotRunning,
}

#[derive(Error, Debug)]
pub enum StartGameError {
    #[error("Game already running")]
    AlreadyRunning,

    #[error("Insufficient players")]
    InsufficientPlayers,
}

pub mod request {
    use futures::{SinkExt, StreamExt};
    use futures::channel::{mpsc, oneshot};
    use futures::task::Poll;

    use crate::engine::players::PlayerId;
    use crate::engine::World;
    use crate::state::{CancelGameError, NoSuchPlayerError, StartGameError};

    pub struct Action<Req, Res> {
        request: Req,
        response: oneshot::Sender<Res>,
    }

    impl<Req, Res> Action<Req, Res> {
        pub fn with_args(request: Req) -> (Self, oneshot::Receiver<Res>) {
            let (tx, rx) = oneshot::channel();
            return (Self { request, response: tx }, rx);
        }
    }

    pub enum Actions {
        StartGame(Action<(), Result<(), StartGameError>>),
        CancelGame(Action<(), Result<(), CancelGameError>>),
        BuzzPlayer(Action<PlayerId, Result<(), NoSuchPlayerError>>),
        KickPlayer(Action<PlayerId, Result<(), NoSuchPlayerError>>),
    }

    #[derive(Clone)]
    pub struct Stub(mpsc::Sender<Actions>);

    impl Stub {
        pub fn create() -> (Stub, mpsc::Receiver<Actions>) {
            let (tx, rx) = mpsc::channel(1);

            return (Stub(tx), rx);
        }

        async fn call<Args, Res>(&mut self, args: Args, f: impl Fn(Action<Args, Res>) -> Actions) -> Res {
            let (action, response) = Action::with_args(args);
            self.0.send(f(action)).await.expect("Sending request");
            return response.await.expect("Receiving response");
        }

        pub async fn start_game(&mut self) -> Result<(), StartGameError> {
            return self.call((), Actions::StartGame).await;
        }

        pub async fn cancel_game(&mut self) -> Result<(), CancelGameError> {
            return self.call((), Actions::CancelGame).await;
        }

        pub async fn buzz_player(&mut self, player: PlayerId) -> Result<(), NoSuchPlayerError> {
            return self.call(player, Actions::BuzzPlayer).await;
        }

        pub async fn kick_player(&mut self, player: PlayerId) -> Result<(), NoSuchPlayerError> {
            return self.call(player, Actions::KickPlayer).await;
        }
    }

    impl super::State {
        pub async fn handle(self, requests: &mut mpsc::Receiver<Actions>, world: &mut World<'_>) -> Self {
            if let Poll::Ready(Some(request)) = futures::poll!(requests.next()) {
                match request {
                    Actions::StartGame(action) => {
                        let (state, result) = self.start(world);
                        action.response.send(result).expect("Sending response");
                        return state;
                    }

                    Actions::CancelGame(action) => {
                        let (state, result) = self.cancel(world);
                        action.response.send(result).expect("Sending response");
                        return state;
                    }

                    Actions::BuzzPlayer(action) => {
                        let (state, result) = self.buzz_player(action.request, world);
                        action.response.send(result).expect("Sending response");
                        return state;
                    }

                    Actions::KickPlayer(action) => {
                        let (state, result) = self.kick_player(action.request, world);
                        action.response.send(result).expect("Sending response");
                        return state;
                    }
                }
            } else {
                return self;
            }
        }
    }
}