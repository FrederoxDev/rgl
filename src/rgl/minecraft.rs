use super::UserConfig;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MinecraftBuild {
    Standard,
    Preview,
    Education,
}

#[cfg(target_os = "linux")]
fn mojang_dir() -> Result<PathBuf> {
    let home = env::var("HOME")?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("mcpelauncher")
        .join("games")
        .join("com.mojang"))
}

#[cfg(target_os = "macos")]
fn mojang_dir() -> Result<PathBuf> {
    let home = env::var("HOME")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("mcpelauncher")
        .join("games")
        .join("com.mojang"))
}

#[cfg(target_os = "windows")]
fn mojang_dir() -> Result<PathBuf> {
    let localappdata = env::var("LocalAppData")?;
    Ok(PathBuf::from(localappdata)
        .join("Packages")
        .join("Microsoft.MinecraftUWP_8wekyb3d8bbwe")
        .join("LocalState")
        .join("games")
        .join("com.mojang"))
}

fn find_standard_mojang_dir() -> Result<PathBuf> {
    if let Some(com_mojang) = UserConfig::mojang_dir() {
        return Ok(PathBuf::from(com_mojang));
    }
    mojang_dir()
}

#[cfg(unix)]
fn find_preview_mojang_dir() -> Result<PathBuf> {
    mojang_dir()
}

#[cfg(target_os = "windows")]
fn find_preview_mojang_dir() -> Result<PathBuf> {
    let localappdata = env::var("LocalAppData")?;
    Ok(PathBuf::from(localappdata)
        .join("Packages")
        .join("Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe")
        .join("LocalState")
        .join("games")
        .join("com.mojang"))
}

#[cfg(unix)]
fn find_education_mojang_dir() -> Result<PathBuf> {
    mojang_dir()
}

#[cfg(target_os = "windows")]
fn find_education_mojang_dir() -> Result<PathBuf> {
    let appdata = env::var("AppData")?;
    Ok(PathBuf::from(appdata)
        .join("Minecraft Education Edition")
        .join("games")
        .join("com.mojang"))
}

pub fn find_mojang_dir(build: Option<&MinecraftBuild>) -> Result<PathBuf> {
    match build {
        Some(MinecraftBuild::Standard) | None => find_standard_mojang_dir(),
        Some(MinecraftBuild::Preview) => find_preview_mojang_dir(),
        Some(MinecraftBuild::Education) => find_education_mojang_dir(),
    }
}

pub fn find_world_dir(build: Option<&MinecraftBuild>, world_name: &str) -> Result<PathBuf> {
    let mojang_dir = find_mojang_dir(build)?;
    if !mojang_dir.exists() {
        bail!("Failed to find com.mojang directory")
    }

    let mut worlds = HashMap::<String, PathBuf>::new();
    for entry in mojang_dir.join("minecraftWorlds").read_dir()? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path();
        let name = std::fs::read_to_string(path.join("levelname.txt"))?;
        if worlds.contains_key(&name) && name == world_name {
            bail!("Found more than one world named <yellow>{name}</>");
        }
        worlds.insert(name, path);
    }

    worlds
        .get(world_name)
        .cloned()
        .with_context(|| format!("World <yellow>{world_name}</> not found"))
}
