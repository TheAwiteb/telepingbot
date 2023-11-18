// A simple API to ping telegram bots and returns if it's online or not.
// Copyright (C) 2023  Awiteb <awitb@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::sync::Arc;

use salvo::{catcher::Catcher, http::HeaderValue, hyper::header, logging::Logger, prelude::*};

use crate::PingList;

#[derive(Debug)]
pub(crate) struct AppState {
    /// Clean text bot usernames
    pub bots: Vec<String>,
    /// Sha256 tokens
    pub tokens: Vec<String>,
    /// The telegram clinet
    tg_client: grammers_client::Client,
}

#[derive(serde::Serialize)]
struct MessageSchema<'a> {
    message: &'a str,
    status: bool,
    #[serde(skip)]
    status_code: StatusCode,
}

impl AppState {
    /// Create new [`AppState`] instance from clean bots and tokens
    pub(crate) fn new(
        bots: Vec<String>,
        tokens: Vec<String>,
        client: grammers_client::Client,
    ) -> Self {
        Self {
            bots: bots
                .into_iter()
                .map(|b| b.trim_start_matches('@').trim().to_lowercase())
                .collect(),
            tokens: tokens
                .into_iter()
                .map(|t| sha256::digest(t.trim()))
                .collect(),
            tg_client: client,
        }
    }
}

impl<'a> MessageSchema<'a> {
    /// Create new [`Message`] instance with `200 OK` status
    fn new(message: &'a str) -> Self {
        Self {
            message,
            status: true,
            status_code: StatusCode::OK,
        }
    }

    /// Update the status code and status
    fn code(mut self, status_code: StatusCode) -> Self {
        self.status = status_code.is_success();
        self.status_code = status_code;
        self
    }
}

fn write_json_body(res: &mut Response, json_body: impl serde::Serialize) {
    res.write_body(serde_json::to_string(&json_body).unwrap())
        .ok();
}

#[handler]
async fn ping(req: &Request, res: &mut Response, depot: &mut Depot) {
    let bot_username = req.param::<String>("bot_username").unwrap().to_lowercase();
    let app_state = depot.obtain::<Arc<AppState>>().unwrap();

    let msg = if !app_state.bots.contains(&bot_username) {
        MessageSchema::new("Is not authorized to check the status of this bot")
            .code(StatusCode::BAD_REQUEST)
    } else if let Ok(telegram_id) =
        crate::superbot::send_start(&app_state.tg_client, &bot_username).await
    {
        if crate::PINGED_BOTS.check(telegram_id) {
            MessageSchema::new("Alive")
        } else {
            MessageSchema::new("No response from the bot").code(StatusCode::NOT_FOUND)
        }
    } else {
        MessageSchema::new("Cant send to the bot").code(StatusCode::INTERNAL_SERVER_ERROR)
    };
    res.status_code(msg.status_code);
    write_json_body(res, msg);
}

#[handler]
async fn handle404(res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(StatusCode::NOT_FOUND) = res.status_code {
        write_json_body(
            res,
            MessageSchema::new("Not Found").code(StatusCode::NOT_FOUND),
        );
        ctrl.skip_rest();
    }
}

#[handler]
async fn handle_server_errors(res: &mut Response, ctrl: &mut FlowCtrl) {
    if matches!(res.status_code, Some(status) if status.is_server_error()) {
        write_json_body(
            res,
            MessageSchema::new("Server Error").code(StatusCode::INTERNAL_SERVER_ERROR),
        );
        ctrl.skip_rest();
    }
}

#[handler]
async fn auth(req: &Request, res: &mut Response, depot: &mut Depot, ctrl: &mut FlowCtrl) {
    let app_state = depot.obtain::<Arc<AppState>>().unwrap();
    log::info!("New auth request");
    if let Some(token) = req.headers().get("Authorization") {
        if let Ok(token) = token.to_str() {
            if app_state.tokens.contains(&sha256::digest(token.trim())) {
                log::info!("The token is authorized");
                return;
            } else {
                log::info!("Unauthorized token");
                write_json_body(
                    res,
                    MessageSchema::new("Unauthorized").code(StatusCode::FORBIDDEN),
                );
            }
        } else {
            log::info!("Invalid token value");
            write_json_body(
                res,
                MessageSchema::new("Invalid token value").code(StatusCode::BAD_REQUEST),
            );
        }
    } else {
        log::info!("Missing `Authorization` header");
        write_json_body(
            res,
            MessageSchema::new("Missing `Authorization` header").code(StatusCode::FORBIDDEN),
        );
    }
    ctrl.skip_rest();
}

#[handler]
async fn add_server_headers(res: &mut Response) {
    let headers = res.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    // Yeah, Rusty programmer
    headers.insert("X-Powered-By", HeaderValue::from_static("Rust/Salvo"));
}

pub(crate) fn service(app_state: AppState) -> Service {
    let router = Router::new()
        .hoop(Logger::new())
        .hoop(affix::inject(Arc::new(app_state)))
        .hoop(add_server_headers)
        .hoop(auth)
        .push(Router::with_path("ping/@<bot_username>").get(ping));
    Service::new(router).catcher(
        Catcher::default()
            .hoop(handle404)
            .hoop(handle_server_errors),
    )
}
