use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "branch",
    about = "Interactive git branch and worktree navigator"
)]
pub struct Cli {
    /// Print shell integration snippet and exit.
    /// Auto-detects shell when omitted. Supported: bash, zsh, fish, powershell.
    #[arg(
        long,
        value_name = "SHELL",
        num_args = 0..=1,
        default_missing_value = "auto"
    )]
    pub init: Option<String>,

    /// List remote-tracking branches instead of local branches
    #[arg(long)]
    pub remote: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn no_args_gives_default_cli() {
        let cli = Cli::parse_from(["branch"]);
        assert!(cli.init.is_none());
        assert!(!cli.remote);
    }

    #[test]
    fn init_auto_is_default_missing_value() {
        let cli = Cli::parse_from(["branch", "--init"]);
        assert_eq!(cli.init.as_deref(), Some("auto"));
    }

    #[test]
    fn init_accepts_explicit_shell() {
        let cli = Cli::parse_from(["branch", "--init", "zsh"]);
        assert_eq!(cli.init.as_deref(), Some("zsh"));
    }

    #[test]
    fn remote_flag_is_parsed() {
        let cli = Cli::parse_from(["branch", "--remote"]);
        assert!(cli.remote);
        assert!(cli.init.is_none());
    }

    #[test]
    fn remote_and_init_can_combine() {
        let cli = Cli::parse_from(["branch", "--remote", "--init", "bash"]);
        assert!(cli.remote);
        assert_eq!(cli.init.as_deref(), Some("bash"));
    }
}
