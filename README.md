# TelePingBot
A simple API to ping telegram bots and returns if it's online or not. using superbot to send message to the bots (mtproto).

## Why is simple?
Add your API tokens in the `tokens.txt` and add the bot usernames in the `bots.txt` and you're ready to go! No need to generate tokens or anything else.

## `tokens.txt` file (rename `tokens.txt.example` to `tokens.txt`)
The `tokens.txt` file is where you put your API tokens. You can put as many as you want, but make sure to put one in each line. This is API access tokens, you need to put it in `Authorization` header.

> [!WARNING]
>
> Remember to keep this file safe, because anyone can use it to ping your bots.
> Recommended to generate the tokens with `openssl rand -hex 32` or `uuidgen`.

## `bots.txt` file (rename `bots.txt.example` to `bots.txt`)
The `bots.txt` file is where you put your bot usernames, this to make sure to ping the specifics bots only. You can put as many as you want, but make sure to put one in each line.

for example:
```
@BotFather
@SomeTestBot
@SomeTestBot
```

## `.env` file (rename `.env.example` to `.env`)
You need to fill the variables in it.

## Requirements
- Rust (MSRV 1.68.2)
- Cargo

## Build
```bash
cargo build --release
```

## Run
```bash
cargo run --release
```
Or just run the binary file in `target/release/telepingbot` (Not recommended because the `.env` file)

## Endpoints

### `/ping/@<bot_username>`
This endpoint is to ping the bot and returns if it's online or not.

#### Headers
- `Authorization`: The API access token. e.g: `Authorization: FirstToken`

#### Response
- `200`: The bot is online.
- `404`: The bot is offline.
- `401`: The API access token is invalid.
- `500`: Internal server error. e.g: The bot username is invalid or the superbot can't send message to the bot.

#### Example
> [!NOTE]
> 
> Replace `FirstToken` with your API access token and `@testbot` with your bot username
> and the host and port with your host and port.

```bash
curl -v 0.0.0.0:3939/ping/@testbot -H "Authorization: FirstToken"
```


