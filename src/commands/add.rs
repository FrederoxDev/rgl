use super::Command;
use crate::rgl::{Config, ConfigCst, RemoteFilter, Session};
use crate::{info, warn};
use anyhow::Result;
use clap::Args;

/// Add filter(s) to current project
#[derive(Args)]
pub struct Add {
    #[arg(required = true)]
    filters: Vec<String>,
    #[arg(short, long, default_missing_value = "default", num_args = 0..)]
    profile: Vec<String>,
    #[arg(short, long)]
    force: bool,
}

impl Command for Add {
    fn dispatch(&self) -> Result<()> {
        let config = Config::load()?;
        let config_cst = ConfigCst::load()?;
        let mut session = Session::lock()?;
        let data_path = config.get_data_path();

        for arg in &self.filters {
            info!("Adding filter <filter>{arg}</>...");
            let (filter_name, remote) = RemoteFilter::parse(arg)?;
            remote.install(&filter_name, Some(&data_path), self.force)?;

            for profile_name in &self.profile {
                if config_cst.add_filter_to_profile(&filter_name, profile_name) {
                    info!("Added filter <filter>{filter_name}</> to <profile>{profile_name}</> profile");
                } else {
                    warn!("Profile <profile>{profile_name}</> not found, skipping...")
                }
            }

            config_cst.add_filter(&filter_name, remote);
            info!("Filter <filter>{filter_name}</> successfully added");
        }

        config_cst.save()?;
        session.unlock()
    }
    fn error_context(&self) -> String {
        "Error adding filter".to_owned()
    }
}
