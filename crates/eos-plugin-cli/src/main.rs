mod inventory;
use eos_plugin_protocol::{error, Result};
use std::path::Path;
async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("version") if args.len() == 2 => {
            println!(
                "{}",
                option_env!("EOS_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
            );
            Ok(())
        }
        Some("inventory") if args.len() == 3 => {
            let report = inventory::inspect(Path::new(&args[2]))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|_| error("SCHEMA_INVALID", "cannot encode inventory"))?
            );
            Ok(())
        }
        Some("compatibility") if args.len() >= 4 => {
            let bytes = std::fs::read(&args[2]).map_err(eos_plugin_protocol::io_error)?;
            let package = eos_plugin_runtime::read_package(&bytes)?;
            let host: eos_plugin_protocol::HostDescriptor = serde_json::from_slice(
                &std::fs::read(&args[3]).map_err(eos_plugin_protocol::io_error)?,
            )
            .map_err(|_| error("SCHEMA_INVALID", "invalid host descriptor"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&eos_plugin_policy::compatible(
                    &package.manifest,
                    &host,
                    &args[4..]
                ))
                .map_err(|_| error("SCHEMA_INVALID", "cannot encode compatibility"))?
            );
            Ok(())
        }
        Some("init-dev") if args.len() == 3 => {
            eos_plugin_runtime::publish::init_development_repository(Path::new(&args[2])).await
        }
        Some("sign") if args.len() >= 8 => {
            let revision = args[5]
                .parse()
                .map_err(|_| error("USAGE", "invalid metadata revision"))?;
            let keys = args[6..]
                .iter()
                .map(std::path::PathBuf::from)
                .collect::<Vec<_>>();
            eos_plugin_runtime::publish::sign_repository(
                Path::new(&args[2]),
                Path::new(&args[3]),
                Path::new(&args[4]),
                revision,
                &keys,
            )
            .await
        }
        Some("pack") if args.len() == 4 => {
            let bytes = eos_plugin_runtime::pack_directory(Path::new(&args[2]))?;
            std::fs::write(&args[3], bytes).map_err(eos_plugin_protocol::io_error)?;
            println!("{}", args[3]);
            Ok(())
        }
        Some("validate") if args.len() == 3 => {
            let bytes = std::fs::read(&args[2]).map_err(eos_plugin_protocol::io_error)?;
            let package = eos_plugin_runtime::read_package(&bytes)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&package.manifest)
                    .map_err(|_| error("SCHEMA_INVALID", "cannot encode manifest"))?
            );
            Ok(())
        }
        _ => Err(error(
            "USAGE",
            "eos-plugin pack <directory> <output.eos-plugin> | validate <artifact>",
        )),
    }
}
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
