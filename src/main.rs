mod browser_auth;
mod cli;
mod client;
mod commands;
mod config;
mod models;
mod render;
mod repl;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use client::LeetCodeClient;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    match cli.command {
        None | Some(Command::Repl) => {
            repl::run(&mut config, cli.cookie.as_deref(), cli.json).await
        }
        Some(command) => {
            let credentials = config.resolve_credentials(cli.cookie.as_deref());
            let client = LeetCodeClient::new(credentials)?;
            dispatch_command(command, &mut config, &client, cli.json).await?;
            Ok(())
        }
    }
}

/// Dispatches a parsed CLI subcommand, optionally returning an updated active problem slug.
pub async fn dispatch_command(
    command: Command,
    config: &mut Config,
    client: &LeetCodeClient,
    json: bool,
) -> Result<Option<String>> {
    match command {
        Command::Repl => unreachable!("Repl command is handled before dispatch"),
        Command::Auth { action } => {
            commands::auth::run(action, config, client, json).await?;
            Ok(None)
        }
        Command::Daily(args) => commands::daily::run(args, config, client, json).await,
        Command::Show(args) => commands::problem::run(args, config, client, json).await,
        Command::List(args) => {
            commands::list::run(args, client, json).await?;
            Ok(None)
        }
        Command::Submissions(args) => {
            commands::submissions::run_list(args, client, json).await?;
            Ok(None)
        }
        Command::Submission(args) => {
            commands::submissions::run_detail(args, client, json).await?;
            Ok(None)
        }
        Command::Stats(args) => {
            commands::stats::run(args, client, json).await?;
            Ok(None)
        }
        Command::Points(args) => {
            commands::points::run(args, client, json).await?;
            Ok(None)
        }
        Command::Run(args) => commands::judge::run_interpret(args, config, client, json).await,
        Command::Submit(args) => commands::judge::run_submit(args, config, client, json).await,
    }
}
