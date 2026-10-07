use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::actions::{LaunchTarget, PasteRequest};
use crate::config::ProfileSet;
use crate::dispatcher::Dispatcher;
use crate::injector::{KeyCombo, KeyInjector};
use crate::protocol::{G13Event, G13Key, MKey};

struct RecordingInjector {
    pastes: Arc<Mutex<Vec<PasteRequest>>>,
    launches: Arc<Mutex<Vec<LaunchTarget>>>,
    downs: Arc<Mutex<u32>>,
    ups: Arc<Mutex<u32>>,
}

impl KeyInjector for RecordingInjector {
    fn press(&self, _: &KeyCombo) -> Result<()> {
        Ok(())
    }
    fn key_down(&self, _: &str) -> Result<()> {
        Ok(())
    }
    fn key_up(&self, _: &str) -> Result<()> {
        Ok(())
    }
    fn combo_down(&self, _: &KeyCombo) -> Result<()> {
        *self.downs.lock().unwrap() += 1;
        Ok(())
    }
    fn combo_up(&self, _: &KeyCombo) -> Result<()> {
        *self.ups.lock().unwrap() += 1;
        Ok(())
    }
    fn paste_text(&self, req: &PasteRequest) -> Result<()> {
        self.pastes.lock().unwrap().push(req.clone());
        Ok(())
    }
    fn launch_or_focus(&self, target: &LaunchTarget) -> Result<()> {
        self.launches.lock().unwrap().push(target.clone());
        Ok(())
    }
}

fn fixture() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("g13-dispatch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("prompts/v1-build")).unwrap();
    std::fs::create_dir_all(dir.join("prompts/workflow-2")).unwrap();
    std::fs::create_dir_all(dir.join("prompts/workflow-3")).unwrap();
    std::fs::create_dir_all(dir.join("profiles")).unwrap();
    std::fs::write(dir.join("prompts/v1-build/01-spec-pr.md"), "step-one\n").unwrap();
    std::fs::write(dir.join("prompts/workflow-2/01-step.md"), "flow-two\n").unwrap();
    std::fs::write(dir.join("prompts/workflow-3/01-step.md"), "flow-three\n").unwrap();
    let profile = |name: &str, file: &str| {
        format!(
            "[meta]\nname = \"{name}\"\n\n[repeat]\nG1 = true\n\n[keys.G1]\n\
type = \"paste_text\"\nfile = \"{file}\"\n\n[keys.BTN2]\n\
type = \"launch_or_focus\"\nexe = 'C:/Tools/Cursor.exe'\n"
        )
    };
    std::fs::write(
        dir.join("profiles/v1.toml"),
        profile("V1 build", "prompts/v1-build/01-spec-pr.md"),
    )
    .unwrap();
    std::fs::write(
        dir.join("profiles/w2.toml"),
        profile("Workflow 2", "prompts/workflow-2/01-step.md"),
    )
    .unwrap();
    std::fs::write(
        dir.join("profiles/w3.toml"),
        profile("Workflow 3", "prompts/workflow-3/01-step.md"),
    )
    .unwrap();
    std::fs::write(
        dir.join("config.toml"),
        "profiles_dir = \"profiles\"\nm1 = \"v1.toml\"\nm2 = \"w2.toml\"\nm3 = \"w3.toml\"\n\
[autorepeat]\ndelay_ms = 1\ninterval_ms = 1\n",
    )
    .unwrap();
    dir
}

#[test]
fn actions_follow_active_profile_and_fire_once() {
    let dir = fixture();
    let set = ProfileSet::load(&dir.join("config.toml")).unwrap();
    assert!(set.slot_errors().is_empty(), "{:?}", set.slot_errors());
    let pastes = Arc::new(Mutex::new(Vec::new()));
    let launches = Arc::new(Mutex::new(Vec::new()));
    let downs = Arc::new(Mutex::new(0u32));
    let ups = Arc::new(Mutex::new(0u32));
    let injector = RecordingInjector {
        pastes: pastes.clone(),
        launches: launches.clone(),
        downs: downs.clone(),
        ups: ups.clone(),
    };
    let mut dispatcher = Dispatcher::new(Arc::new(RwLock::new(set)), Box::new(injector));
    dispatcher.handle(G13Event::MKeyDown(MKey::M1)).unwrap();
    dispatcher.handle(G13Event::KeyDown(G13Key::G1)).unwrap();
    dispatcher.tick(Instant::now() + Duration::from_secs(2));
    dispatcher.handle(G13Event::KeyUp(G13Key::G1)).unwrap();
    {
        let got = pastes.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].file_label, "prompts/v1-build/01-spec-pr.md");
        assert_eq!(got[0].text, "step-one\n");
    }
    dispatcher.handle(G13Event::MKeyDown(MKey::M2)).unwrap();
    dispatcher.handle(G13Event::KeyDown(G13Key::G1)).unwrap();
    dispatcher.handle(G13Event::KeyUp(G13Key::G1)).unwrap();
    assert_eq!(
        pastes.lock().unwrap()[1].file_label,
        "prompts/workflow-2/01-step.md"
    );
    assert_eq!(pastes.lock().unwrap()[1].text, "flow-two\n");
    for slot in [MKey::M1, MKey::M2, MKey::M3] {
        dispatcher.handle(G13Event::MKeyDown(slot)).unwrap();
        dispatcher.handle(G13Event::KeyDown(G13Key::Btn2)).unwrap();
        dispatcher.handle(G13Event::KeyUp(G13Key::Btn2)).unwrap();
    }
    let launches = launches.lock().unwrap();
    assert_eq!(launches.len(), 3);
    assert!(launches.iter().all(|l| l.process_name == "Cursor.exe"));
    assert_eq!(*downs.lock().unwrap(), 0);
    assert_eq!(*ups.lock().unwrap(), 0);
    assert_eq!(pastes.lock().unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}
