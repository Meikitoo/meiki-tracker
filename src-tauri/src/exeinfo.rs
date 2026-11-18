//! Informations lues dans un fichier .exe (sur le disque, pas en mémoire) :
//! son icône et son nom « lisible » (description du fichier).
//! Et la liste des processus qui ont une fenêtre visible (comme l'onglet
//! « Applications » du Gestionnaire des tâches). Tout est en lecture seule.

use base64::Engine;
use std::collections::HashSet;
use std::path::Path;
use windows::core::{BOOL, HSTRING};
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::Graphics::Gdi::{
    BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW, HBITMAP,
};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::Win32::UI::Shell::SHDefExtractIconW;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, EnumWindows, GetIconInfo, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId,
    GW_OWNER, HICON, ICONINFO, IsWindowVisible,
};

const ICON_SIZE: u32 = 64;

/// Icône principale du .exe, encodée en PNG.
pub fn icon_png(exe: &Path) -> Option<Vec<u8>> {
    let mut hicon = HICON::default();
    let path = HSTRING::from(exe.as_os_str());
    unsafe {
        SHDefExtractIconW(&path, 0, 0, Some(&mut hicon), None, ICON_SIZE).ok().ok()?;
    }
    if hicon.is_invalid() {
        return None;
    }
    let result = unsafe { hicon_to_png(hicon) };
    unsafe {
        let _ = DestroyIcon(hicon);
    }
    result
}

/// PNG → « data URL » utilisable directement dans une balise <img>.
pub fn png_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

unsafe fn hicon_to_png(hicon: HICON) -> Option<Vec<u8>> {
    let mut info = ICONINFO::default();
    unsafe { GetIconInfo(hicon, &mut info).ok()? };
    let color = info.hbmColor;
    let mask = info.hbmMask;
    let pixels = if color.is_invalid() { None } else { unsafe { bitmap_bgra(color) } };
    unsafe {
        if !color.is_invalid() {
            let _ = DeleteObject(color.into());
        }
        if !mask.is_invalid() {
            let _ = DeleteObject(mask.into());
        }
    }
    let (w, h, mut px) = pixels?;

    // BGRA → RGBA. Les très vieilles icônes n'ont pas de transparence : on les rend opaques.
    let has_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
    for p in px.chunks_exact_mut(4) {
        p.swap(0, 2);
        if !has_alpha {
            p[3] = 255;
        }
    }

    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(&px).ok()?;
    }
    Some(out)
}

unsafe fn bitmap_bgra(hbm: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut bm = BITMAP::default();
    let got = unsafe {
        GetObjectW(hbm.into(), size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _))
    };
    if got == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
        return None;
    }
    let (w, h) = (bm.bmWidth as u32, bm.bmHeight as u32);
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32), // négatif = lignes de haut en bas
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let lines = unsafe {
        let dc = CreateCompatibleDC(None);
        let n = GetDIBits(dc, hbm, 0, h, Some(buf.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
        let _ = DeleteDC(dc);
        n
    };
    (lines > 0).then_some((w, h, buf))
}

/// Description du fichier (ex. « Call of Duty® »), lue dans ses informations de version.
pub fn file_description(exe: &Path) -> Option<String> {
    let path = HSTRING::from(exe.as_os_str());
    unsafe {
        let size = GetFileVersionInfoSizeW(&path, None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(&path, None, size, data.as_mut_ptr() as *mut _).ok()?;

        // Langue/page de code déclarées par le fichier.
        let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let mut langs: Vec<(u16, u16)> = Vec::new();
        if VerQueryValueW(data.as_ptr() as *const _, &HSTRING::from(r"\VarFileInfo\Translation"), &mut ptr, &mut len)
            .as_bool()
            && !ptr.is_null()
        {
            let pairs = std::slice::from_raw_parts(ptr as *const u16, (len / 2) as usize);
            langs.extend(pairs.chunks_exact(2).map(|c| (c[0], c[1])));
        }
        langs.extend([(0x0409, 0x04B0), (0x0409, 0x04E4), (0x040C, 0x04B0)]);

        for key in ["FileDescription", "ProductName"] {
            for (lang, cp) in &langs {
                let sub = HSTRING::from(format!(r"\StringFileInfo\{lang:04x}{cp:04x}\{key}"));
                if VerQueryValueW(data.as_ptr() as *const _, &sub, &mut ptr, &mut len).as_bool()
                    && !ptr.is_null()
                    && len > 1
                {
                    let s = std::slice::from_raw_parts(ptr as *const u16, len as usize);
                    let text = String::from_utf16_lossy(s).trim_matches(char::from(0)).trim().to_string();
                    if !text.is_empty() {
                        return Some(clean_name(&text));
                    }
                }
            }
        }
        None
    }
}

/// Retire les ® ™ ©, décode les entités HTML courantes et les espaces superflus.
fn clean_name(s: &str) -> String {
    s.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace(['®', '™', '©'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Nom par défaut tiré du nom du fichier : « FortniteClient-Win64-Shipping.exe » → « FortniteClient ».
pub fn name_from_file(process_name: &str) -> String {
    let stem = process_name.strip_suffix(".exe").or_else(|| process_name.strip_suffix(".EXE")).unwrap_or(process_name);
    let stem = stem.split("-Win64").next().unwrap_or(stem);
    let stem = stem.trim_end_matches("_x64").trim_end_matches("64");
    if stem.is_empty() { process_name.to_string() } else { stem.to_string() }
}

/// PID des processus qui possèdent une fenêtre principale visible avec un titre.
pub fn windowed_pids() -> HashSet<u32> {
    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let set = &mut *(lparam.0 as *mut HashSet<u32>);
            let is_main = GetWindow(hwnd, GW_OWNER).map(|o| o.is_invalid()).unwrap_or(true);
            if IsWindowVisible(hwnd).as_bool() && is_main && GetWindowTextLengthW(hwnd) > 0 {
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid != 0 {
                    set.insert(pid);
                }
            }
        }
        BOOL(1) // continuer l'énumération
    }
    let mut set: HashSet<u32> = HashSet::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut set as *mut _ as isize));
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_par_defaut() {
        assert_eq!(name_from_file("FortniteClient-Win64-Shipping.exe"), "FortniteClient");
        assert_eq!(name_from_file("cod.exe"), "cod");
        assert_eq!(name_from_file("obs64.exe"), "obs");
        assert_eq!(clean_name("Call of Duty®  HQ"), "Call of Duty HQ");
    }

    #[test]
    fn icone_et_description_d_un_exe_windows() {
        let notepad = Path::new(r"C:\Windows\System32\notepad.exe");
        if notepad.exists() {
            let png = icon_png(notepad).expect("icône extraite");
            assert_eq!(&png[1..4], b"PNG");
            assert!(file_description(notepad).is_some());
        }
    }
}
