use std::io;
use std::path::PathBuf;

use clap::Parser;
use directories::ProjectDirs;

use crate::Cache;

#[derive(Parser, Debug, Clone)]
pub struct CacheArgs {
    /// Avoid reading from or writing to the cache.
    #[arg(
        global = true,
        long,
        short,
        alias = "no-cache-dir",
        env = "UV_NO_CACHE"
    )]
    pub no_cache: Option<bool>,

    /// Path to the cache directory.
    ///
    /// Defaults to `$HOME/Library/Caches/uv` on macOS, `$XDG_CACHE_HOME/uv` or `$HOME/.cache/uv` on
    /// Linux, and `$HOME/.cache/<project_path> {FOLDERID_LocalAppData}/<project_path>/cache/uv`
    /// on Windows.
    #[arg(global = true, long, env = "UV_CACHE_DIR")]
    pub cache_dir: Option<PathBuf>,
}

impl Cache {
    /// Prefer, in order:
    /// 1. A temporary cache directory, if the user requested `--no-cache`.
    /// 2. The specific cache directory specified by the user via `--cache-dir` or `UV_CACHE_DIR`.
    /// 3. The system-appropriate cache directory.
    /// 4. A `.uv_cache` directory in the current working directory.
    ///
    /// Returns an absolute cache dir.
    pub fn from_settings(
        no_cache: Option<bool>,
        cache_dir: Option<PathBuf>,
    ) -> Result<Self, io::Error> {
        if no_cache.unwrap_or(false) {
            Cache::temp()
        } else if let Some(cache_dir) = cache_dir {
            Cache::from_path(cache_dir)
        } else if let Some(project_dirs) = ProjectDirs::from("", "", "uv") {
            Cache::from_path(project_dirs.cache_dir())
        } else {
            Cache::from_path(".uv_cache")
        }
    }
}

impl TryFrom<CacheArgs> for Cache {
    type Error = io::Error;

    /// Prefer, in order:
    /// 1. A temporary cache directory, if the user requested `--no-cache`.
    /// 2. The specific cache directory specified by the user via `--cache-dir` or `UV_CACHE_DIR`.
    /// 3. The system-appropriate cache directory.
    /// 4. A `.uv_cache` directory in the current working directory.
    ///
    /// Returns an absolute cache dir.
    fn try_from(value: CacheArgs) -> Result<Self, Self::Error> {
        Cache::from_settings(value.no_cache, value.cache_dir)
    }
}
