use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct SelfUpdateArgs {
    /// Update to the specified version. If not provided, mdns-scanner will update to the latest version.
    pub target_version: Option<String>,

    /// A GitHub token for authentication.
    /// A token is not required but can be used to reduce the chance of encountering rate limits.
    #[arg(long, env = "MDS_GITHUB_TOKEN")]
    pub token: Option<String>,

    /// Run without performing the update.
    #[arg(long)]
    pub dry_run: bool,
}
