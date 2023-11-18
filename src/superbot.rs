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

use grammers_client::{Client, Config, InitParams, SignInError, Update};
use grammers_session::Session;

use crate::PingList;

const SESSION_FILE: &str = "telebotping.session";

pub(crate) async fn login(api_hash: String, api_id: i32) -> crate::Result<(Client, bool)> {
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id,
        api_hash: api_hash.clone(),
        params: InitParams::default(),
    })
    .await?;
    let mut sign_out = false;

    if !client.is_authorized().await? {
        println!("Signing in...");
        let phone: String = promptly::prompt("Enter your phone number (international format)")?;
        let token = client.request_login_code(&phone, api_id, &api_hash).await?;
        let code: String = promptly::prompt("Enter the code you received")?;
        let signed_in = client.sign_in(&token, &code).await;
        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                let hint = password_token.hint().unwrap_or("None");
                let password: String =
                    promptly::prompt(format!("Enter the password (hint {hint})"))?;
                client
                    .check_password(password_token, password.trim())
                    .await?;
            }
            Ok(_) => (),
            Err(e) => panic!("{e}"),
        }
        let me = client.get_me().await?;
        println!(
            "Signed in successfully to {}",
            me.username()
                .map(|u| "@".to_owned() + u)
                .unwrap_or_else(|| me.full_name())
        );
        match client.session().save_to_file(SESSION_FILE) {
            Ok(_) => {}
            Err(e) => {
                println!(
                    "NOTE: failed to save the session, will sign out when done: {}",
                    e
                );
                sign_out = true;
            }
        }
    }

    Ok((client, sign_out))
}

fn update_handler(upd: Update) {
    if let Update::NewMessage(msg) = upd {
        if let Some(sender) = msg.sender() {
            crate::PINGED_BOTS.new_res(sender.id() as u64)
        }
    }
}

pub(crate) async fn handler(client: Client) {
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            Ok(Some(update)) = client.next_update() => {
                log::debug!("New update: {update:?}");
                tokio::spawn(async move {
                    update_handler(update)
                });
            }
        }
    }
}

pub(crate) async fn send_start(client: &Client, bot_username: &str) -> crate::Result<u64> {
    if let Some(chat) = client.resolve_username(bot_username).await? {
        let telegram_id = chat.id() as u64;
        crate::PINGED_BOTS.add_new(telegram_id);
        client.send_message(chat, "/start").await?;
        // Sleep, wating the response
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Ok(telegram_id)
    } else {
        Err(format!("Invalid username `{bot_username}`").into())
    }
}
