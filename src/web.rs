use std::future::Future;
use std::net::SocketAddr;

use anyhow::Result;
use futures::channel::mpsc;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tracing::info;
use warp::{body, Filter, get, http, log, path, post, reject, Rejection, Reply, reply};

use crate::engine::players::PlayerId;
use crate::GAME_MODE;
use crate::games::GameMode;
use crate::state::{StartGameError, CancelGameError, NoSuchPlayerError};
use crate::state::request::{Stub, Actions};

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

#[derive(Serialize, Deserialize)]
pub struct GameModePayload {
    pub mode: GameMode,
}

impl reject::Reject for StartGameError {}

impl reject::Reject for CancelGameError {}

impl reject::Reject for NoSuchPlayerError {}

fn mode_get() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return get()
        .and(path!("mode"))
        .map(move || {
            let mode = GameModePayload { mode: *GAME_MODE.lock() };
            return Ok(reply::json(&mode));
        });
}

fn mode_set() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    return post()
        .and(path!("mode"))
        .and(body::json())
        .map(move |body: GameModePayload| {
            *GAME_MODE.lock() = body.mode;
            return Ok(http::StatusCode::OK);
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

pub fn serve() -> Result<(impl Future<Output=()>, mpsc::Receiver<Actions>)> {
    let addr: SocketAddr = "0.0.0.0:3000".parse()?;

    let (stub, requests) = Stub::create();

    let api = mode_get()
        .or(mode_set())
        .or(game_start(stub.clone()))
        .or(game_cancel(stub.clone()))
        .or(player_buzz(stub.clone()))
        .or(player_kick(stub.clone()));

    let api = path("api")
        .and(api)
        .with(log::log("api"));

    let routes = Filter::or(
        Static::route(),
        api);

    let server = warp::serve(routes).run(addr);

    info!("Web-Server listening on {}", addr);

    return Ok((server, requests));
}