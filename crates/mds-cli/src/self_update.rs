use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct SelfUpdateArgs {
    /// Update to the specified version. If not provided, mdns-scanner will update to the latest version.
    pub target_version: Option<String>,

    /// Run without performing the update.
    #[arg(long)]
    pub dry_run: bool,
}
