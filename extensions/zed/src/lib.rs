use zed_extension_api::{self as zed, LanguageServerId, Result};

struct LidExtension;

impl zed::Extension for LidExtension {
    fn new() -> Self {
        LidExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let path = worktree.which("lid-lsp").ok_or_else(|| {
            concat!(
                "lid-lsp not found on PATH.\n",
                "Install via Homebrew:  brew install lid-tooling\n",
                "or via curl:  curl -fsSL ",
                "https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.sh",
                " | bash",
            )
            .to_string()
        })?;

        Ok(zed::Command {
            command: path,
            args: Vec::new(),
            env: Default::default(),
        })
    }
}

zed::register_extension!(LidExtension);
