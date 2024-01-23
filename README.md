# RoWifi - The 2nd Gen Roblox-Discord Verification Bot
Highly customizable bot written in Rust to make your Discord server's integration with your Roblox's group extremely flexible.

**STILL UNDER DEVELOPMENT**

This is version 4 of RoWifi. It is being rewritten from scratch to follow best software development practices and stuff I've learned over the last few years.

## What you need to run this
- Rust (nightly channel)
- Docker
- Redis
- PostgreSQL

## How to run this
You will find a list of environment variables to set in [here](https://github.com/RoWifi-HQ/RoWifi-V3/blob/master/rowifi/src/main.rs). You will also need the **Guild Members** Intent found on the Discord Developers Dashboard.

If you're running this locally, you can just do
```sh
cargo run # to run a development build
# or
cargo run --release # to run a release build
```