use zed_extension_api::{self as zed, LanguageServerId, Worktree};

struct RpmSpecExtension {
    cached_binary_path: Option<String>,
}

impl RpmSpecExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<String> {
        // First check if the binary is already available on PATH
        if let Some(path) = worktree.which("rpm-spec-ls") {
            return Ok(path);
        }

        // Return cached path if it still exists on disk
        if let Some(ref path) = self.cached_binary_path {
            if std::path::Path::new(path).exists() {
                return Ok(path.clone());
            }
        }

        // Determine the download URL based on the current platform
        let (os, arch) = zed::current_platform();
        let binary_name = match os {
            zed::Os::Windows => "rpm-spec-ls.exe",
            _ => "rpm-spec-ls",
        };
        let platform_suffix = match (os, arch) {
            (zed::Os::Linux, zed::Architecture::X8664) => "linux-x86_64",
            (zed::Os::Mac, zed::Architecture::X8664) => "macos-x86_64",
            (zed::Os::Mac, zed::Architecture::Aarch64) => "macos-aarch64",
            _ => {
                return Err(format!(
                    "rpm-spec-ls: unsupported platform {:?}/{:?}",
                    os, arch
                ))
            }
        };

        let version = "0.1.0";
        let download_url = format!(
            "https://github.com/mtalexan/zed-rpm-spec-extension/releases/download/v{version}/rpm-spec-ls-{platform_suffix}"
        );

        let binary_path = format!("{}/{binary_name}", language_server_id.as_ref());

        zed::download_file(
            &download_url,
            &binary_path,
            zed::DownloadedFileType::Uncompressed,
        )
        .map_err(|e| format!("rpm-spec-ls: failed to download binary: {e}"))?;

        zed::make_file_executable(&binary_path)
            .map_err(|e| format!("rpm-spec-ls: failed to make binary executable: {e}"))?;

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for RpmSpecExtension {
    fn new() -> Self {
        RpmSpecExtension {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        let binary = self.language_server_binary_path(language_server_id, worktree)?;
        Ok(zed::Command {
            command: binary,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(RpmSpecExtension);
