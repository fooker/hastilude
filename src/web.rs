use std::collections::HashSet;
use std::future::Future;
use std::net::SocketAddr;

use anyhow::Result;
use futures::channel::mpsc;
use futures::SinkExt;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize, Serializer};
use tokio::sync::watch;
use tracing::info;
use warp::{body, Filter, get, http, log, path, post, reject, Rejection, Reply};
use warp::ws;

use crate::controller::{Address, Battery, Controller, Model};
use crate::engine::players::PlayerId;
use crate::games::GameMode;
use crate::state::{CancelGameError, NoSuchPlayerError, StartGameError, State};
use crate::state::request::{Actions, Stub};

#[derive(RustEmbed)]
#[folder = "web/dist/"]
struct Static;

impl Static {
    async fn serve(path: &str) -> Result<impl Reply, Rejection> {
        let asset = Self::get(path)
            .ok_or_else(reject::not_found)?;

        let mime = mime_guess::from_path(path).first_or_octet_stream();

        return Ok(http::Response::builder()
            .header("Content-Type", mime.as_ref())
            .body(asset.data));
    }

    pub async fn serve_index() -> Result<impl Reply, Rejection> {
        return Self::serve("index.html").await;
    }

    pub async fn serve_asset(path: path::Tail) -> Result<impl Reply, Rejection> {
        return Self::serve(path.as_str()).await;
    }

    pub fn route() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
        let index = path::end().and_then(Self::serve_index);
        let asset = path::tail().and_then(Self::serve_asset);

        return get()
            .and(Filter::or(index, asset));
    }
}

#[derive(Serialize, Clone, PartialEq)]
pub enum GameStateDTO {
    Waiting {
        ready: HashSet<PlayerId>,
    },

    Running {},
}

impl From<&State> for GameStateDTO {
    fn from(state: &State) -> Self {
        return match state {
            State::Lobby(lobby) => Self::Waiting {
                ready: lobby.ready().clone(),
            },
            State::Countdown(_) => Self::Running {},
            State::Playing(_) => Self::Running {},
            State::Celebration(_) => Self::Running {},
        };
    }
}

#[derive(Serialize, Clone, PartialEq)]
pub struct ControllerInfoDTO {
    pub address: Address,
    pub signal: f64,
    pub battery: Battery,
    pub model: Model,
}

impl From<&Controller> for ControllerInfoDTO {
    fn from(controller: &Controller) -> Self {
        return Self {
            address: controller.serial(),
            signal: 0.0,
            battery: controller.battery(),
            model: controller.model(),
        };
    }
}

#[derive(Serialize, Clone, PartialEq)]
pub struct StateDTO {
    pub mode: GameModeDTO,
    pub state: GameStateDTO,
    pub devices: Vec<ControllerInfoDTO>,
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.as_string())
    }
}

impl Default for StateDTO {
    fn default() -> Self {
        return Self {
            mode: Default::default(),
            state: GameStateDTO::Waiting {
                ready: Default::default(),
            },
            devices: Default::default(),
        };
    }
}

pub struct InfoPublisher(watch::Sender<StateDTO>);

impl InfoPublisher {
    pub fn publish(&mut self, info: StateDTO) {
        if *self.0.borrow() != info {
            self.0.send_replace(info);
        }
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Eq, Copy, Clone)]
pub struct GameModeDTO {
    pub mode: GameMode,
}

impl From<GameMode> for GameModeDTO {
    fn from(mode: GameMode) -> Self {
        return Self {
            mode,
        };
    }
}

impl reject::Reject for StartGameError {}

impl reject::Reject for CancelGameError {}

impl reject::Reject for NoSuchPlayerError {}

fn mode_set(stub: Stub) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return post()
        .map(move || stub.clone())
        .and(path!("mode"))
        .and(body::json())
        .then(|mut stub: Stub, body: GameModeDTO| async move {
            stub.game_mode(body.mode).await;
            return http::StatusCode::OK;
        });
}

fn game_start(stub: Stub) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return post()
        .map(move || stub.clone())
        .and(path!("game" / "start"))
        .and_then(|mut stub: Stub| async move {
            return match stub.start_game().await {
                Ok(()) => Ok(http::StatusCode::OK),
                Err(err) => Err(reject::custom(err)),
            };
        });
}

fn game_cancel(stub: Stub) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return post()
        .map(move || stub.clone())
        .and(path!("game" / "cancel"))
        .and_then(|mut stub: Stub| async move {
            return match stub.cancel_game().await {
                Ok(()) => Ok(http::StatusCode::OK),
                Err(err) => Err(reject::custom(err)),
            };
        });
}

fn player_buzz(stub: Stub) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return post()
        .map(move || stub.clone())
        .and(path!("player" / PlayerId / "buzz"))
        .and_then(|mut stub: Stub, player_id: PlayerId| async move {
            return match stub.buzz_player(player_id).await {
                Ok(()) => Ok(http::StatusCode::OK),
                Err(err) => Err(reject::custom(err)),
            };
        });
}

fn player_kick(stub: Stub) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return post()
        .map(move || stub.clone())
        .and(path!("game" / PlayerId / "kick"))
        .and_then(|mut stub: Stub, player_id: PlayerId| async move {
            return match stub.kick_player(player_id).await {
                Ok(()) => Ok(http::StatusCode::OK),
                Err(err) => Err(reject::custom(err)),
            };
        });
}

fn state(rx: watch::Receiver<StateDTO>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return ws()
        .and(path!("state"))
        .map(move |ws: ws::Ws| {
            let mut rx = rx.clone();
            ws.on_upgrade(|mut ws| async move {
                loop {
                    let info = rx.borrow_and_update().clone();
                    let info = serde_json::to_string(&info)
                        .expect("Failed to serialize state message");

                    if let Err(_) = ws.send(ws::Message::text(info)).await {
                        break;
                    }

                    if let Err(_) = rx.changed().await {
                        break;
                    }
                }
            })
        });
}

pub fn serve() -> Result<(impl Future<Output=()>, mpsc::Receiver<Actions>, InfoPublisher)> {
    let addr: SocketAddr = "0.0.0.0:3000".parse()?;

    let (stub, requests) = Stub::create();

    let (info_publisher, info_watch) = watch::channel(StateDTO::default());
    let info_publisher = InfoPublisher(info_publisher);

    let api = mode_set(stub.clone())
        .or(game_start(stub.clone()))
        .or(game_cancel(stub.clone()))
        .or(player_buzz(stub.clone()))
        .or(player_kick(stub.clone()))
        .or(state(info_watch));

    let api = path("api")
        .and(api)
        .with(log::log("api"));

    let routes = Filter::or(
        Static::route(),
        api);

    let server = warp::serve(routes).run(addr);

    info!("Web-Server listening on {}", addr);

    return Ok((server, requests, info_publisher));
}