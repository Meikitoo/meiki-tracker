//! Lecture de l'état du PC, en lecture seule :
//! - liste des processus ouverts (comme le Gestionnaire des tâches) ;
//! - temps depuis la dernière action clavier/souris/manette.
//!
//! Aucune interaction avec les jeux : pas de lecture mémoire, pas d'injection, pas de hook.

use std::collections::HashSet;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Un programme ouvert, tel que proposé dans « Ajouter un jeu ».
#[derive(Debug, Clone)]
pub struct RunningProgram {
    pub process_name: String,
    pub exe_path: Option<String>,
}

pub struct ProcessScanner {
    sys: Option<System>,
}

impl ProcessScanner {
    pub fn new() -> Self {
        ProcessScanner { sys: System::new().ok() }
    }

    /// Accès au système ; réessaie l'initialisation si elle avait échoué.
    fn sys(&mut self) -> Option<&mut System> {
        if self.sys.is_none() {
            self.sys = System::new().ok();
        }
        self.sys.as_mut()
    }

    /// Noms (en minuscules) des processus ouverts. Ne lit que la liste, rien d'autre.
    /// `None` si la liste n'a pas pu être lue : il ne faut alors surtout pas
    /// conclure que les jeux sont fermés.
    pub fn running_names(&mut self) -> Option<HashSet<String>> {
        let sys = self.sys()?;
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        let names: HashSet<String> = sys
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_lowercase())
            .collect();
        (!names.is_empty()).then_some(names)
    }

    /// Programmes ouverts, hors liste d'exclusion, sans doublons, triés par nom.
    pub fn candidate_programs(&mut self) -> Vec<RunningProgram> {
        let Some(sys) = self.sys() else { return Vec::new() };
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );
        let mut seen = HashSet::new();
        let mut out: Vec<RunningProgram> = sys
            .processes()
            .values()
            .filter_map(|p| {
                let name = p.name().to_string_lossy().to_string();
                let exe = p.exe().map(|e| e.to_string_lossy().to_string());
                let lower = name.to_lowercase();
                let in_windows_dir = exe
                    .as_deref()
                    .is_some_and(|e| e.to_lowercase().starts_with(r"c:\windows\"));
                if !lower.ends_with(".exe")
                    || crate::exclusions::is_excluded(&lower)
                    || in_windows_dir
                    || !seen.insert(lower)
                {
                    return None;
                }
                Some(RunningProgram { process_name: name, exe_path: exe })
            })
            .collect();
        out.sort_by_key(|p| p.process_name.to_lowercase());
        out
    }
}

impl Default for ProcessScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Mesure l'inactivité du joueur. Clavier/souris via `GetLastInputInfo` ;
/// manettes Xbox (et compatibles XInput) via `XInputGetState`, en lecture seule.
pub struct InputMonitor {
    pads: [Option<u32>; 4],
    last_pad_activity: Option<i64>,
}

impl InputMonitor {
    pub fn new() -> Self {
        InputMonitor { pads: [None; 4], last_pad_activity: None }
    }

    /// Secondes depuis la dernière action, tous périphériques confondus.
    pub fn idle_secs(&mut self, now: i64) -> i64 {
        let kbd = keyboard_mouse_idle_secs();
        if self.poll_pads() {
            self.last_pad_activity = Some(now);
        }
        match self.last_pad_activity {
            Some(t) => kbd.min((now - t).max(0)),
            None => kbd,
        }
    }

    /// `true` si une manette a changé d'état depuis la dernière vérification.
    fn poll_pads(&mut self) -> bool {
        use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
        let mut changed = false;
        for (i, slot) in self.pads.iter_mut().enumerate() {
            let mut state = XINPUT_STATE::default();
            // 0 = ERROR_SUCCESS (manette branchée)
            let ok = unsafe { XInputGetState(i as u32, &mut state) } == 0;
            let packet = ok.then_some(state.dwPacketNumber);
            if let (Some(prev), Some(cur)) = (*slot, packet) {
                if prev != cur {
                    changed = true;
                }
            }
            *slot = packet;
        }
        changed
    }
}

impl Default for InputMonitor {
    fn default() -> Self {
        Self::new()
    }
}

fn keyboard_mouse_idle_secs() -> i64 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    unsafe {
        if !GetLastInputInfo(&mut info).as_bool() {
            return 0; // en cas d'échec, on suppose le joueur présent
        }
        // wrapping_sub : le compteur Windows repart à zéro tous les ~49 jours.
        (GetTickCount().wrapping_sub(info.dwTime) / 1000) as i64
    }
}
