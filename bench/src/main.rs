// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

#![cfg(not(target_os = "android"))]
#[pollster::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = vellobench::Cli::parse();
    vellobench::run(&cli).await
}
