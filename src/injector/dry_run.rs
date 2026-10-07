//! Records action lines for `--simulate`. Prompt text is never written.

use std::sync::{Arc, Mutex};

use anyhow::Result;

use super::{KeyCombo, KeyInjector};
use crate::actions::{LaunchTarget, PasteRequest};

pub struct DryRunInjector {
    lines: Arc<Mutex<Vec<String>>>,
}

impl DryRunInjector {
    pub fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let lines = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                lines: lines.clone(),
            },
            lines,
        )
    }
}

impl KeyInjector for DryRunInjector {
    fn press(&self, _combo: &KeyCombo) -> Result<()> {
        Ok(())
    }
    fn key_down(&self, _key: &str) -> Result<()> {
        Ok(())
    }
    fn key_up(&self, _key: &str) -> Result<()> {
        Ok(())
    }
    fn combo_down(&self, _combo: &KeyCombo) -> Result<()> {
        Ok(())
    }
    fn combo_up(&self, _combo: &KeyCombo) -> Result<()> {
        Ok(())
    }

    fn paste_text(&self, req: &PasteRequest) -> Result<()> {
        let chars = req.text.chars().count();
        self.lines.lock().unwrap().push(format!(
            "[dry-run] {} -> paste_text file={} method={} restore_clipboard={} chars={chars}",
            req.key_name,
            req.file_label,
            req.method.as_str(),
            req.restore_clipboard
        ));
        Ok(())
    }

    fn launch_or_focus(&self, target: &LaunchTarget) -> Result<()> {
        self.lines.lock().unwrap().push(format!(
            "[dry-run] {} -> launch_or_focus process={} exe={}",
            target.key_name, target.process_name, target.exe
        ));
        Ok(())
    }
}
