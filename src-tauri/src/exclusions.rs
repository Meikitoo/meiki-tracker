//! Processus qui ne doivent jamais être proposés comme jeux :
//! launchers, anti-triches, composants Windows et outils courants.

const EXCLUDED: &[&str] = &[
    // Launchers
    "battle.net.exe", "agent.exe", "blizzard error.exe", "blizzardbrowser.exe",
    "steam.exe", "steamwebhelper.exe", "steamservice.exe", "gameoverlayui.exe",
    "epicgameslauncher.exe", "epicwebhelper.exe", "epiconlineservices.exe",
    "epiconlineserviceshost.exe", "epiconlineservicesuserhelper.exe",
    "eadesktop.exe", "eabackgroundservice.exe", "ealocalhostsvc.exe", "origin.exe",
    "originwebhelperservice.exe", "ubisoftconnect.exe", "upc.exe", "uplaywebcore.exe",
    "ubisoftgamelauncher.exe", "galaxyclient.exe", "galaxyclient helper.exe",
    "galaxycommunication.exe", "riotclientservices.exe", "riotclientux.exe",
    "riotclientuxrender.exe", "riotclientcrashhandler.exe", "leagueclient.exe",
    "leagueclientux.exe", "leagueclientuxrender.exe", "rockstarservice.exe",
    "launcher.exe", "socialclubhelper.exe", "playnite.desktopapp.exe",
    "xboxpcapp.exe", "xboxappservices.exe", "gamingservices.exe", "gamingservicesnet.exe",
    "gamebar.exe", "gamebarftserver.exe", "gamebarpresencewriter.exe",
    "amazon games ui.exe", "itch.exe", "heroic.exe",
    // Anti-triches
    "easyanticheat.exe", "easyanticheat_eos.exe", "easyanticheat_eos_setup.exe",
    "beservice.exe", "beservice_x64.exe", "battleye launcher.exe", "vgc.exe", "vgtray.exe",
    "faceit.exe", "faceitclient.exe", "faceitservice.exe", "esportal.exe",
    "gamemon.des", "gamemon64.des", "xigncode.exe", "nprotect.exe",
    "randgrid.exe", "anticheatexpert.exe", "ace-tray.exe",
    // Windows
    "system", "system idle process", "idle", "registry", "memory compression",
    "secure system", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe",
    "services.exe", "lsass.exe", "lsaiso.exe", "svchost.exe", "fontdrvhost.exe",
    "dwm.exe", "explorer.exe", "sihost.exe", "taskhostw.exe", "runtimebroker.exe",
    "searchhost.exe", "searchindexer.exe", "searchprotocolhost.exe", "searchfilterhost.exe",
    "startmenuexperiencehost.exe", "shellexperiencehost.exe", "shellhost.exe",
    "textinputhost.exe", "ctfmon.exe", "conhost.exe", "dllhost.exe", "audiodg.exe",
    "spoolsv.exe", "wmiprvse.exe", "dashost.exe", "wudfhost.exe", "smartscreen.exe",
    "securityhealthservice.exe", "securityhealthsystray.exe", "msmpeng.exe", "nissrv.exe",
    "mpdefendercoreservice.exe", "applicationframehost.exe", "systemsettings.exe",
    "systemsettingsbroker.exe", "lockapp.exe", "useroobebroker.exe", "widgets.exe",
    "widgetservice.exe", "phoneexperiencehost.exe", "crossdeviceservice.exe",
    "backgroundtaskhost.exe", "compattelrunner.exe", "taskmgr.exe", "mmc.exe",
    "msedgewebview2.exe", "identity_helper.exe", "sppsvc.exe", "trustedinstaller.exe",
    "tiworker.exe", "msiexec.exe", "werfault.exe", "wermgr.exe", "consent.exe",
    "cmd.exe", "powershell.exe", "pwsh.exe", "windowsterminal.exe", "openconsole.exe",
    "unsecapp.exe", "wlanext.exe", "midisrv.exe", "aggregatorhost.exe", "lsm.exe",
    "onedrive.exe", "filecoauth.exe", "officeclicktorun.exe",
    // Pilotes et utilitaires matériels
    "nvcontainer.exe", "nvdisplay.container.exe", "nvidia share.exe", "nvidia overlay.exe",
    "nvidia app.exe", "nvsphelper64.exe", "nvbroadcast.container.exe",
    "amdrsserv.exe", "radeonsoftware.exe", "amdow.exe", "atieclxx.exe", "atiesrxx.exe",
    "igfxem.exe", "igfxcuiservice.exe", "rtkauduservice64.exe", "realtekaudioservice.exe",
    "lghub.exe", "lghub_agent.exe", "lghub_updater.exe", "icue.exe", "razer synapse.exe",
    "razer synapse service.exe", "armourycrate.exe", "steelseriesgg.exe",
    // Meiki Tracker lui-même
    "meiki-tracker.exe", "meiki-debug.exe",
];

/// `true` si ce nom de processus ne doit jamais être proposé comme jeu.
pub fn is_excluded(process_name: &str) -> bool {
    let name = process_name.to_lowercase();
    EXCLUDED.contains(&name.as_str())
        || name.contains("crashhandler")
        || name.ends_with("crashreporter.exe")
        || name.contains("updater")
        || name.starts_with("unins")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launchers_et_systeme_exclus() {
        assert!(is_excluded("Battle.net.exe"));
        assert!(is_excluded("steam.exe"));
        assert!(is_excluded("EasyAntiCheat_EOS.exe"));
        assert!(is_excluded("svchost.exe"));
        assert!(is_excluded("UnityCrashHandler64.exe"));
        assert!(is_excluded("Discord_Updater.exe"));
    }

    #[test]
    fn jeux_non_exclus() {
        assert!(!is_excluded("cod.exe"));
        assert!(!is_excluded("FortniteClient-Win64-Shipping.exe"));
        assert!(!is_excluded("obs64.exe"));
    }
}
