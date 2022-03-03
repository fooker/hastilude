use std::future::Future;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::{AddExtensionLayer, Json, Router, Server};
use axum::body::{boxed, Empty, Full};
use axum::extract::{Extension, Path};
use axum::http::{header, Response};
use axum::response::IntoResponse;
use axum::routing::{get, MethodRouter, post};
use futures::{SinkExt, TryFutureExt};
use futures::channel::mpsc;
use hyper::{StatusCode, Uri};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::engine::players::PlayerId;
use crate::GAME_MODE;
use crate::games::GameMode;

#[derive(RustEmbed)]
#[folder = "web/dist/"]
struct Static;

impl Static {
    pub async fn handle(uri: Uri) -> impl IntoResponse {
        let mut path = uri.path().trim_start_matches('/');
        if path == "" {
            path = "index.html";
        }

        match Self::get(path) {
            Some(content) => {
                let body = boxed(Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                return Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .expect("Invalid response");
            }

            None => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(boxed(Empty::default()))
                    .expect("Invalid response");
            }
        }
    }
}

pub enum CancelGameResponse {
    Canceled,
    NotRunning,
}

pub enum StartGameResponse {
    Started,
    AlreadyRunning,
    NotEnoughPlayers,
}

pub enum KickPlayerResponse {
    Kicked,
    NotFound,
    AlreadyDead,
}

pub enum BuzzPlayerResponse {
    Buzzed,
    NotFound,
}

pub enum Action {
    CancelGame {
        response: futures::channel::oneshot::Sender<CancelGameResponse>,
    },

    StartGame {
        response: futures::channel::oneshot::Sender<StartGameResponse>,
    },

    KickPlayer {
        player: PlayerId,
        response: futures::channel::oneshot::Sender<KickPlayerResponse>,
    },

    BuzzPlayer {
        player: PlayerId,
        response: futures::channel::oneshot::Sender<BuzzPlayerResponse>,
    },
}

#[derive(Clone)]
struct State {
    actions: mpsc::Sender<Action>,
}

#[derive(Serialize, Deserialize)]
pub struct ModePayload {
    pub mode: String,
}

async fn mode_get() -> impl IntoResponse {
    return Json(ModePayload {
        mode: GAME_MODE.lock().to_string()
    });
}

async fn mode_set(Json(payload): Json<ModePayload>) -> Result<impl IntoResponse, (StatusCode, String)> {
    *GAME_MODE.lock() = GameMode::from_str(&payload.mode)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;
    return Ok(());
}

async fn game_start(Extension(mut state): Extension<State>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (response, result) = futures::channel::oneshot::channel();
    state.actions.send(Action::StartGame { response }).await
        .map_err(|_| ((StatusCode::SERVICE_UNAVAILABLE, "Request not send")))?;
    return match result.await {
        Ok(StartGameResponse::Started) => Ok(()),
        Ok(StartGameResponse::AlreadyRunning) => Err((StatusCode::CONFLICT, "Already running")),
        Ok(StartGameResponse::NotEnoughPlayers) => Err((StatusCode::NOT_ACCEPTABLE, "Not enough players")),
        Err(_) => Err((StatusCode::SERVICE_UNAVAILABLE, "Request canceled")),
    };
}

async fn game_cancel(Extension(mut state): Extension<State>) -> impl IntoResponse {
    let (response, result) = futures::channel::oneshot::channel();
    state.actions.send(Action::CancelGame { response }).await
        .map_err(|_| ((StatusCode::SERVICE_UNAVAILABLE, "Request not send")))?;
    return match result.await {
        Ok(CancelGameResponse::Canceled) => Ok(()),
        Ok(CancelGameResponse::NotRunning) => Err((StatusCode::CONFLICT, "Not running")),
        Err(_) => Err((StatusCode::SERVICE_UNAVAILABLE, "Request canceled")),
    };
}

async fn player_buzz(Extension(mut state): Extension<State>, Path(id): Path<PlayerId>) -> impl IntoResponse {
    let (response, result) = futures::channel::oneshot::channel();
    state.actions.send(Action::BuzzPlayer { player: id, response }).await
        .map_err(|_| ((StatusCode::SERVICE_UNAVAILABLE, "Request not send")))?;
    return match result.await {
        Ok(BuzzPlayerResponse::Buzzed) => Ok(()),
        Ok(BuzzPlayerResponse::NotFound) => Err((StatusCode::NOT_FOUND, "No such player")),
        Err(_) => Err((StatusCode::SERVICE_UNAVAILABLE, "Request canceled")),
    };
}

async fn player_kick(Extension(mut state): Extension<State>, Path(id): Path<PlayerId>) -> impl IntoResponse {
    let (response, result) = futures::channel::oneshot::channel();
    state.actions.send(Action::KickPlayer { player: id, response }).await
        .map_err(|_| ((StatusCode::SERVICE_UNAVAILABLE, "Request not send")))?;
    return match result.await {
        Ok(KickPlayerResponse::Kicked) => Ok(()),
        Ok(KickPlayerResponse::NotFound) => Err((StatusCode::NOT_FOUND, "No such player")),
        Ok(KickPlayerResponse::AlreadyDead) => Err((StatusCode::CONFLICT, "Already dead")),
        Err(_) => Err((StatusCode::SERVICE_UNAVAILABLE, "Request canceled"))
    };
}

fn api() -> (Router, mpsc::Receiver<Action>) {
    let (actions_tx, actions_rx) = mpsc::channel(1);

    let shared = State {
        actions: actions_tx,
    };

    return (Router::new()
                .layer(AddExtensionLayer::new(Arc::new(shared)))
                .route("/mode", MethodRouter::new()
                    .get(mode_get)
                    .post(mode_set))
                .route("/game/start", post(game_start))
                .route("/game/cancel", post(game_cancel))
                .route("/players/:id/buzz", post(player_buzz))
                .route("/players/:id/kick", post(player_kick))
            , actions_rx);
}

pub fn serve() -> Result<(mpsc::Receiver<Action>, impl Future<Output=Result<()>>)> {
    let (api, actions_rx) = api();

    let app = Router::new()
        .nest("/api", api)
        .fallback(get(Static::handle));

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;

    info!("Web-Server listening on {}", addr);

    return Ok((actions_rx,
               Server::try_bind(&addr)?
                   .serve(app.into_make_service())
                   .map_err(Into::into)));
}