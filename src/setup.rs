use mds_config::AppConfig;
use mds_keybindings::KeyBindings;

pub(super) fn setup() -> color_eyre::Result<Option<(AppConfig, KeyBindings)>> {
    let arg_count = std::env::args().count();
    let cfg = if arg_count == 1 {
        mds_config::AppConfig::load()
    } else {
        let args = mds_cli::parse_cli_args();
        if let Some(cmd) = args.command() {
            match cmd {
                mds_cli::cli::Commands::DumpDefaultConfig { output } => {
                    if let Some(output) = output {
                        std::fs::write(output, AppConfig::default_config())?
                    } else {
                        print!("{}", AppConfig::default_config());
                    }
                    return Ok(None);
                }
                mds_cli::cli::Commands::DumpDefaultKeymap { output } => {
                    if let Some(output) = output {
                        std::fs::write(output, KeyBindings::default_keymap())?
                    } else {
                        print!("{}", KeyBindings::default_keymap());
                    }
                    return Ok(None);
                }
                mds_cli::cli::Commands::CheckKeymap { file } => {
                    match KeyBindings::validate_and_report(file) {
                        Ok(report) => println!("{report}"),
                        Err(e) => {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                    }
                    return Ok(None);
                }

                #[cfg(feature = "self-update")]
                mds_cli::cli::Commands::Update(self_update_args) => {
                    crate::self_update::run_self_update(
                        self_update_args.target_version,
                        self_update_args.token,
                        self_update_args.dry_run,
                    )?;
                    return Ok(None);
                }
            }
        }
        mds_config::AppConfig::load_with_cli(&args)
    }?;
    let keybindings = KeyBindings::load()?;
    Ok(Some((cfg, keybindings)))
}
