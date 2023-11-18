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

use std::{env, fs, sync::Mutex};

use lazy_static::lazy_static;
use salvo::Listener;

mod api;
mod superbot;

#[derive(Default, Clone)]
pub(crate) struct PingedBot {
    telegram_id: u64,
    ping_in: i64,
    is_response: bool,
}

pub(crate) trait PingList {
    fn clear_outdead(&self);
    fn add_new(&self, telegram_id: u64);
    fn check(&self, telegram_id: u64) -> bool;
    fn new_res(&self, telegram_id: u64);
}

impl PingList for Mutex<Vec<PingedBot>> {
    fn clear_outdead(&self) {
        log::info!("Clear the dead pings");
        let dead_time = chrono::Utc::now().timestamp() - 60;
        let mut bots = self.lock().unwrap();
        *bots = bots
            .iter()
            .filter(|b| b.ping_in > dead_time)
            .cloned()
            .collect();
    }

    fn add_new(&self, telegram_id: u64) {
        log::debug!("Adding new bot to the list: {telegram_id}");
        self.lock().unwrap().push(PingedBot::new(telegram_id));
    }

    fn check(&self, telegram_id: u64) -> bool {
        log::debug!("Checking the {telegram_id} if is response");
        self.clear_outdead();
        let result = self
            .lock()
            .unwrap()
            .iter()
            .any(|b| b.telegram_id == telegram_id && b.is_response);
        log::debug!("Response status: {result}");
        result
    }
    fn new_res(&self, telegram_id: u64) {
        log::debug!("New res from: {telegram_id}");
        let mut bots = self.lock().unwrap();
        *bots = bots
            .iter()
            .cloned()
            .map(|b| {
                if b.telegram_id == telegram_id {
                    log::info!("Found the sender in the list");
                    b.new_res()
                } else {
                    b
                }
            })
            .collect();
    }
}

impl PingedBot {
    pub(crate) fn new(telegram_id: u64) -> Self {
        Self {
            telegram_id,
            ping_in: chrono::Utc::now().timestamp(),
            is_response: false,
        }
    }

    pub(crate) fn new_res(mut self) -> Self {
        self.is_response = true;
        self
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

lazy_static! {
    static ref PINGED_BOTS: Mutex<Vec<PingedBot>> = Mutex::new(Vec::new());
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    dotenv::dotenv().ok();
    log::info!("Starting the API");

    let bots: Vec<String> = fs::read_to_string("bots.txt")?
        .lines()
        .map(|b| b.trim().to_owned())
        .collect();
    let tokens: Vec<String> = fs::read_to_string("tokens.txt")?
        .lines()
        .map(|b| b.trim().to_owned())
        .collect();

    if bots
        .iter()
        .any(|b| !b.starts_with('@') || !b.to_lowercase().ends_with("bot"))
    {
        bots.iter().for_each(|b| {
            if !b.starts_with('@') {
                eprintln!("Invalid bot username `{b}`: must starts with `@`");
            } else if !b.to_lowercase().ends_with("bot") {
                eprintln!("Invalid bot username `{b}`: must end with `bot`");
            }
        })
    } else {
        let (client, sign_out) = superbot::login(
            env::var("TELEPINGBOT_API_HASH")
                .expect("`TELEPINGBOT_API_HASH` environment variable is required"),
            env::var("TELEPINGBOT_API_ID")
                .expect("`TELEPINGBOT_API_ID` environment variable is required")
                .parse()
                .expect("Invalid value for `TELEPINGBOT_API_ID` must be a number"),
        )
        .await?;
        let host = env::var("TELEOINGBOT_HOST")
            .expect("`TELEOINGBOT_HOST` environment variable must be set");
        let port = env::var("TELEOINGBOT_PORT")
            .expect("`TELEOINGBOT_PORT` environment variable must be set");
        let app_state = api::AppState::new(bots, tokens, client.clone());

        let handler_client = client.clone();
        let acceptor = salvo::conn::TcpListener::new(format!("{host}:{port}"))
            .bind()
            .await;
        let client_handler = tokio::spawn(async move { superbot::handler(handler_client).await });
        let server_handler = tokio::spawn(async move {
            salvo::Server::new(acceptor)
                .serve_with_graceful_shutdown(
                    api::service(app_state),
                    async {
                        tokio::signal::ctrl_c()
                            .await
                            .expect("Faild to listen to ctrl_c event");
                    },
                    None,
                )
                .await
        });

        client_handler.await?;
        server_handler.await?;

        log::debug!("Close the API, telegram sign out status: {sign_out}");
        if sign_out {
            client.sign_out_disconnect().await?;
        }
    }
    Ok(())
}
