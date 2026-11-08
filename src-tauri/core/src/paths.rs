//! Where Achaja keeps things. Everything lives in the install folder:
//!   <install>\Achaja.exe, <install>\resources\ (rules for Claude, Vosk library, templates)
//!   <install>\data\  config.json, .mcp.json, .claude\ (the Claude workspace), state\, models\, webview\
//! The data folder is also the working folder of the hidden Claude session, so the relative paths
//! in its rules (state\..., config.json) keep working. ACHAJA_DATA / ACHAJA_RESOURCES override the
//! folders in development; nothing is ever written to %APPDATA% or %LOCALAPPDATA%.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Paths {
    pub exe: PathBuf,
    pub install: PathBuf,
    pub data: PathBuf,
    pub resources: PathBuf,
}

static PATHS: OnceLock<Paths> = OnceLock::new();

impl Paths {
    fn detect() -> Paths {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("Achaja.exe"));
        let install = exe.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        let data = std::env::var_os("ACHAJA_DATA").map(PathBuf::from).unwrap_or_else(|| install.join("data"));
        let resources = std::env::var_os("ACHAJA_RESOURCES")
            .map(PathBuf::from)
            .unwrap_or_else(|| install.join("resources"));
        Paths { exe, install, data, resources }
    }

    /// Tests and tools point everything at one folder.
    pub fn at(data: &Path, resources: &Path) -> Paths {
        Paths {
            exe: PathBuf::from("Achaja.exe"),
            install: data.to_path_buf(),
            data: data.to_path_buf(),
            resources: resources.to_path_buf(),
        }
    }

    pub fn state(&self) -> PathBuf {
        self.data.join("state")
    }
    pub fn state_file(&self, name: &str) -> PathBuf {
        self.state().join(name)
    }
    pub fn agents(&self) -> PathBuf {
        self.state().join("agents")
    }
    pub fn logs(&self) -> PathBuf {
        self.data.join("logs")
    }
    pub fn models(&self) -> PathBuf {
        self.data.join("models")
    }
    pub fn config(&self) -> PathBuf {
        self.data.join("config.json")
    }
    pub fn webview(&self) -> PathBuf {
        self.data.join("webview")
    }
    pub fn claude_dir(&self) -> PathBuf {
        self.data.join(".claude")
    }
    pub fn mcp(&self) -> PathBuf {
        self.data.join(".mcp.json")
    }
    pub fn ha_token(&self) -> PathBuf {
        self.state_file("ha_token.txt")
    }
    /// Rules, skills, templates and libvosk shipped with the program: "vosk/libvosk.dll" ->
    /// resources\vosk\libvosk.dll, joined part by part (Windows' DLL search with
    /// LOAD_WITH_ALTERED_SEARCH_PATH fails on a "/" in the path).
    pub fn bundled(&self, rel: &str) -> PathBuf {
        rel.split(['/', '\\']).filter(|p| !p.is_empty()).fold(self.resources.clone(), |acc, part| acc.join(part))
    }
}

/// The folders of this run (detected once from the executable's place).
pub fn paths() -> &'static Paths {
    PATHS.get_or_init(Paths::detect)
}

/// Used by tests and by the app to fix the folders before anything reads them.
pub fn set_paths(p: Paths) {
    let _ = PATHS.set(p);
}

/// Creates the data folders; the error names the folder that cannot be written.
pub fn ensure_dirs(p: &Paths) -> std::io::Result<()> {
    for dir in [p.data.clone(), p.state(), p.agents(), p.models()] {
        std::fs::create_dir_all(&dir)
            .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {e}", dir.display())))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_paths_have_no_forward_slash() {
        let p = Paths::at(Path::new("/data"), Path::new("/res"));
        let sep = std::path::MAIN_SEPARATOR;
        let want = format!("/res{sep}vosk{sep}libvosk.dll");
        assert_eq!(p.bundled("vosk/libvosk.dll").to_string_lossy(), want);
        assert_eq!(p.bundled("vosk\\libvosk.dll").to_string_lossy(), want);
    }
}
