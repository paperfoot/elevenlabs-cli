#![recursion_limit = "512"]
//! elevenlabs -- agent-friendly CLI for the ElevenLabs AI audio platform.
//!
//! Built on the agent-cli-framework patterns:
//!   - JSON envelope on stdout, coloured human output on TTY
//!   - Semantic exit codes (0-4)
//!   - `agent-info` capability manifest
//!   - Self-install to Claude/Codex/Gemini
//!   - Self-update from GitHub Releases

mod cli;
mod client;
mod commands;
mod config;
mod error;
mod output;

use clap::Parser;

use cli::{Cli, Commands, ConfigAction, SkillAction};
use output::{Ctx, Format};

/// Pre-scan argv for `--json` before clap parses. This ensures `--json` is
/// honoured even on help, version, and parse-error paths where clap hasn't
/// populated the Cli struct yet.
fn has_json_flag() -> bool {
    std::env::args_os().any(|a| a == "--json")
}

fn main() {
    let json_flag = has_json_flag();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // --help and --version are not errors. Exit 0.
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let format = Format::detect(json_flag);
                match format {
                    Format::Json => {
                        output::print_help_json(e);
                        std::process::exit(0);
                    }
                    Format::Human => e.exit(),
                }
            }

            // Actual parse errors: we own the exit code, not clap. Always 3.
            let format = Format::detect(json_flag);
            output::print_clap_error(format, &e);
            std::process::exit(3);
        }
    };

    let ctx = Ctx::new(cli.json, cli.quiet);

    // Construct a Tokio runtime once for commands that need HTTP.
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            output::print_error(
                ctx.format,
                &error::AppError::Transient(format!("failed to start runtime: {e}")),
            );
            std::process::exit(1);
        }
    };

    let result = rt.block_on(async move {
        match cli.command {
            // Meta / framework commands
            Commands::AgentInfo => {
                commands::agent_info::run();
                Ok(())
            }
            Commands::Skill { action } => match action {
                SkillAction::Install => commands::skill::install(ctx),
                SkillAction::Status => commands::skill::status(ctx),
            },
            Commands::Config { action } => match action {
                ConfigAction::Show => {
                    let cfg = config::load()?;
                    commands::config::show(ctx, &cfg)
                }
                ConfigAction::Path => commands::config::path(ctx),
                ConfigAction::Set { key, value } => commands::config::set(ctx, &key, &value),
                ConfigAction::Check => {
                    let cfg = config::load()?;
                    commands::config::check(ctx, &cfg).await
                }
                ConfigAction::Init { api_key } => commands::config::init(ctx, api_key),
            },
            Commands::Update { check } => {
                let cfg = config::load()?;
                commands::update::run(ctx, check, &cfg)
            }

            // Domain commands
            Commands::Tts(args) => commands::tts::run(ctx, args).await,
            Commands::Stt(args) => commands::stt::run(ctx, *args).await,
            Commands::Sfx(args) => commands::sfx::run(ctx, args).await,
            Commands::Voices { action } => commands::voices::dispatch(ctx, action).await,
            Commands::Models { action } => commands::models::dispatch(ctx, action).await,
            Commands::Audio { action } => commands::audio::dispatch(ctx, action).await,
            Commands::Music { action } => commands::music::dispatch(ctx, action).await,
            Commands::User { action } => commands::user::dispatch(ctx, action).await,
            Commands::Agents { action } => commands::agents::dispatch(ctx, action).await,
            Commands::Conversations { action } => {
                commands::conversations::dispatch(ctx, action).await
            }
            Commands::Phone { action } => commands::phone::dispatch(ctx, action).await,
            Commands::History { action } => commands::history::dispatch(ctx, action).await,
        }
    });

    if let Err(e) = result {
        output::print_error(ctx.format, &e);
        std::process::exit(e.exit_code());
    }
}
