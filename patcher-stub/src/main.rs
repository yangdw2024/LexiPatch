#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, GetStockObject, HFONT, DEFAULT_CHARSET, DEFAULT_GUI_FONT,
    OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH, FF_DONTCARE,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ, REG_SZ,
};
use windows_sys::Win32::UI::Controls::{
    INITCOMMONCONTROLSEX, InitCommonControlsEx, ICC_STANDARD_CLASSES,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, GetWindowTextLengthW, GetWindowTextW, MessageBoxW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetWindowTextW, ShowWindow, TranslateMessage,
    BM_SETSTYLE, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, ES_AUTOHSCROLL, ES_MULTILINE, ES_READONLY,
    ES_AUTOVSCROLL, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNO, IDYES,
    MSG, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_SETFONT, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX, WS_OVERLAPPED,
    WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

const MAGIC: &[u8; 8] = b"TCPATCH2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchFileEntry {
    /// 补丁包内相对路径（也是目标软件目录内的相对路径）
    pub target_path: String,
    /// 原文件哈希（可选，用于校验）
    pub original_sha256: Option<String>,
    /// 汉化后文件哈希
    pub patched_sha256: String,
    /// 词条统计
    pub unit_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchManifest {
    pub patch_id: String,
    pub project_name: String,
    pub software_name: String,
    pub version: String,
    pub author: String,
    pub note: String,
    pub exe_name: Option<String>,
    pub default_paths: Vec<String>,
    pub created_at: String,
    pub files: Vec<PatchFileEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupRecord {
    patch_id: String,
    software_name: String,
    version: String,
    backed_up_at: String,
    /// target_path -> true if original existed and was backed up, false if file was newly created
    entries: HashMap<String, bool>,
}

struct EmbeddedPayload {
    manifest: PatchManifest,
    zip_bytes: Vec<u8>,
}

static PAYLOAD: OnceLock<Result<EmbeddedPayload, String>> = OnceLock::new();

fn load_embedded_payload() -> Result<&'static EmbeddedPayload, String> {
    let res = PAYLOAD.get_or_init(|| {
        let exe_path = env::current_exe().map_err(|e| format!("无法获取自身路径: {e}"))?;
        let mut f = fs::File::open(&exe_path).map_err(|e| format!("无法读取补丁文件: {e}"))?;
        let total_len = f.metadata().map_err(|e| e.to_string())?.len();
        if total_len < 16 {
            return Err("当前程序未嵌入任何汉化补丁数据（此为通用补丁引导器）。".into());
        }
        f.seek(SeekFrom::End(-16)).map_err(|e| e.to_string())?;
        let mut footer = [0u8; 16];
        f.read_exact(&mut footer).map_err(|e| e.to_string())?;
        if &footer[8..16] != MAGIC {
            return Err("当前程序未嵌入任何汉化补丁数据（魔数校验失败）。".into());
        }
        let payload_len = u64::from_le_bytes(footer[0..8].try_into().unwrap());
        if payload_len == 0 || payload_len + 16 > total_len {
            return Err("补丁数据长度异常，文件可能已损坏。".into());
        }
        f.seek(SeekFrom::End(-(16 + payload_len as i64)))
            .map_err(|e| e.to_string())?;
        let mut zip_bytes = vec![0u8; payload_len as usize];
        f.read_exact(&mut zip_bytes).map_err(|e| e.to_string())?;

        let manifest: PatchManifest = {
            let reader = Cursor::new(&zip_bytes);
            let mut archive =
                zip::ZipArchive::new(reader).map_err(|e| format!("补丁包解压失败: {e}"))?;
            let mut manifest_file = archive
                .by_name("patch_manifest.json")
                .map_err(|_| "补丁包缺少 patch_manifest.json".to_string())?;
            let mut manifest_str = String::new();
            manifest_file
                .read_to_string(&mut manifest_str)
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&manifest_str)
                .map_err(|e| format!("清单解析失败: {e}"))?
        };

        Ok(EmbeddedPayload {
            manifest,
            zip_bytes,
        })
    });
    res.as_ref().map_err(|e| e.clone())
}

fn expand_env_vars(input: &str) -> String {
    let mut out = input.to_string();
    for (k, v) in env::vars() {
        let pattern = format!("%{}%", k);
        if out.to_ascii_lowercase().contains(&pattern.to_ascii_lowercase()) {
            // Case-insensitive replace
            let lower_out = out.to_ascii_lowercase();
            let lower_pat = pattern.to_ascii_lowercase();
            let mut result = String::new();
            let mut idx = 0;
            while let Some(pos) = lower_out[idx..].find(&lower_pat) {
                result.push_str(&out[idx..idx + pos]);
                result.push_str(&v);
                idx += pos + lower_pat.len();
            }
            result.push_str(&out[idx..]);
            out = result;
        }
    }
    out
}

fn read_reg_string(hkey: HKEY, subkey: &str, value_name: &str) -> Option<String> {
    unsafe {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(Some(0)).collect();
        let mut key_handle: HKEY = null_mut();
        if RegOpenKeyExW(hkey, subkey_w.as_ptr(), 0, KEY_READ, &mut key_handle) != 0 {
            return None;
        }
        let val_w: Vec<u16> = value_name.encode_utf16().chain(Some(0)).collect();
        let mut reg_type: u32 = 0;
        let mut buf_size: u32 = 0;
        let res = RegQueryValueExW(
            key_handle,
            val_w.as_ptr(),
            null(),
            &mut reg_type,
            null_mut(),
            &mut buf_size,
        );
        if res != 0 || buf_size == 0 || (reg_type != REG_SZ && reg_type != REG_EXPAND_SZ) {
            RegCloseKey(key_handle);
            return None;
        }
        let mut buf = vec![0u16; (buf_size as usize / 2) + 1];
        let res2 = RegQueryValueExW(
            key_handle,
            val_w.as_ptr(),
            null(),
            &mut reg_type,
            buf.as_mut_ptr() as *mut u8,
            &mut buf_size,
        );
        RegCloseKey(key_handle);
        if res2 != 0 {
            return None;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..end]))
    }
}

fn find_uninstall_location(software_name: &str) -> Option<PathBuf> {
    let target_lower = software_name.to_ascii_lowercase();
    let roots = [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER];
    let uninstall_paths = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];
    for root in roots {
        for base in uninstall_paths {
            unsafe {
                let base_w: Vec<u16> = base.encode_utf16().chain(Some(0)).collect();
                let mut hkey: HKEY = null_mut();
                if RegOpenKeyExW(root, base_w.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
                    continue;
                }
                let mut idx = 0u32;
                loop {
                    let mut name_buf = [0u16; 256];
                    let mut name_len = name_buf.len() as u32;
                    let r = RegEnumKeyExW(
                        hkey,
                        idx,
                        name_buf.as_mut_ptr(),
                        &mut name_len,
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                    );
                    if r != 0 {
                        break;
                    }
                    idx += 1;
                    let sub_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                    let full_sub = format!("{base}\\{sub_name}");
                    if let Some(display_name) = read_reg_string(root, &full_sub, "DisplayName") {
                        if display_name.to_ascii_lowercase().contains(&target_lower) {
                            if let Some(loc) = read_reg_string(root, &full_sub, "InstallLocation") {
                                let p = PathBuf::from(loc.trim().trim_matches('"'));
                                if p.is_dir() {
                                    RegCloseKey(hkey);
                                    return Some(p);
                                }
                            }
                        }
                    }
                }
                RegCloseKey(hkey);
            }
        }
    }
    None
}

fn auto_detect_dir(manifest: &PatchManifest) -> Option<PathBuf> {
    // 1. Check current exe directory (if user dropped patcher inside software folder)
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            if manifest.files.iter().any(|f| dir.join(&f.target_path).exists()) {
                return Some(dir.to_path_buf());
            }
        }
    }
    // 2. Check configured default paths
    for raw in &manifest.default_paths {
        let expanded = expand_env_vars(raw);
        let p = PathBuf::from(expanded);
        if p.is_dir() {
            return Some(p);
        }
    }
    // 3. Check App Paths by exe_name
    if let Some(exe_name) = &manifest.exe_name {
        if !exe_name.trim().is_empty() {
            let sub = format!(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{}",
                exe_name.trim()
            );
            for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
                if let Some(val) = read_reg_string(root, &sub, "") {
                    let p = PathBuf::from(val.trim().trim_matches('"'));
                    if let Some(parent) = p.parent() {
                        if parent.is_dir() {
                            return Some(parent.to_path_buf());
                        }
                    }
                }
            }
        }
    }
    // 4. Check Uninstall registry by software_name
    if !manifest.software_name.trim().is_empty() {
        if let Some(loc) = find_uninstall_location(&manifest.software_name) {
            return Some(loc);
        }
    }
    None
}

fn backup_dir(target_dir: &Path, patch_id: &str) -> PathBuf {
    target_dir.join(".transcore_backup").join(patch_id)
}

fn apply_patch(payload: &EmbeddedPayload, target_dir: &Path) -> Result<String, String> {
    if !target_dir.is_dir() {
        return Err("目标目录不存在，请先选择有效的软件安装目录。".into());
    }

    let bdir = backup_dir(target_dir, &payload.manifest.patch_id);
    let bmanifest_path = bdir.join("backup_manifest.json");
    let has_existing_backup = bmanifest_path.exists();

    let mut backup_entries: HashMap<String, bool> = if has_existing_backup {
        let txt = fs::read_to_string(&bmanifest_path).unwrap_or_default();
        serde_json::from_str::<BackupRecord>(&txt)
            .map(|r| r.entries)
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let reader = Cursor::new(&payload.zip_bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("读取补丁压缩包失败: {e}"))?;

    let mut logs = Vec::new();
    let mut applied_count = 0usize;

    for entry in &payload.manifest.files {
        let dest = target_dir.join(&entry.target_path);
        // Backup original if not already backed up
        if !backup_entries.contains_key(&entry.target_path) {
            if dest.exists() {
                let bak_dest = bdir.join(&entry.target_path);
                if let Some(parent) = bak_dest.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("创建备份目录失败: {e}"))?;
                }
                fs::copy(&dest, &bak_dest)
                    .map_err(|e| format!("备份原文件 {} 失败: {e}", entry.target_path))?;
                backup_entries.insert(entry.target_path.clone(), true);
                logs.push(format!("[备份] {}", entry.target_path));
            } else {
                backup_entries.insert(entry.target_path.clone(), false);
            }
        }

        // Extract patched file
        let zip_name = format!("files/{}", entry.target_path.replace('\\', "/"));
        let mut zf = archive
            .by_name(&zip_name)
            .map_err(|_| format!("补丁包内缺失文件: {zip_name}"))?;
        let mut buf = Vec::new();
        zf.read_to_end(&mut buf).map_err(|e| e.to_string())?;

        // Verify hash
        let actual_hash = hex::encode(Sha256::digest(&buf));
        if actual_hash != entry.patched_sha256 {
            return Err(format!("文件 {} 校验失败，补丁可能已损坏。", entry.target_path));
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目标目录失败: {e}"))?;
        }
        fs::write(&dest, &buf)
            .map_err(|e| format!("写入文件 {} 失败（请检查是否需要管理员权限或先关闭目标软件）: {e}", entry.target_path))?;
        applied_count += 1;
        logs.push(format!("[汉化] {} ({} 词条)", entry.target_path, entry.unit_count));
    }

    fs::create_dir_all(&bdir).map_err(|e| format!("创建备份元数据目录失败: {e}"))?;
    let record = BackupRecord {
        patch_id: payload.manifest.patch_id.clone(),
        software_name: payload.manifest.software_name.clone(),
        version: payload.manifest.version.clone(),
        backed_up_at: "saved".into(),
        entries: backup_entries,
    };
    fs::write(&bmanifest_path, serde_json::to_string_pretty(&record).unwrap())
        .map_err(|e| format!("写入备份清单失败: {e}"))?;

    logs.push(format!("\r\n成功应用 {} 个汉化文件！原版已备份至 .transcore_backup 目录。", applied_count));
    Ok(logs.join("\r\n"))
}

fn restore_backup(payload: &EmbeddedPayload, target_dir: &Path) -> Result<String, String> {
    let bdir = backup_dir(target_dir, &payload.manifest.patch_id);
    let bmanifest_path = bdir.join("backup_manifest.json");
    if !bmanifest_path.exists() {
        return Err("未在该目录下找到此补丁的原版备份记录。".into());
    }
    let txt = fs::read_to_string(&bmanifest_path).map_err(|e| e.to_string())?;
    let record: BackupRecord = serde_json::from_str(&txt).map_err(|e| e.to_string())?;

    let mut logs = Vec::new();
    let mut restored = 0usize;

    for (rel, existed) in &record.entries {
        let dest = target_dir.join(rel);
        if *existed {
            let bak = bdir.join(rel);
            if bak.exists() {
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                fs::copy(&bak, &dest)
                    .map_err(|e| format!("还原 {} 失败: {e}", rel))?;
                logs.push(format!("[还原] {}", rel));
                restored += 1;
            }
        } else if dest.exists() {
            let _ = fs::remove_file(&dest);
            logs.push(format!("[清理新增] {}", rel));
            restored += 1;
        }
    }

    let _ = fs::remove_dir_all(&bdir);
    logs.push(format!("\r\n已成功还原 {} 个文件至原版状态！", restored));
    Ok(logs.join("\r\n"))
}

// ============== Win32 Native GUI ==============

const ID_EDIT_DIR: usize = 101;
const ID_BTN_BROWSE: usize = 102;
const ID_EDIT_LOG: usize = 103;
const ID_BTN_INSTALL: usize = 104;
const ID_BTN_RESTORE: usize = 105;
const ID_BTN_CLOSE: usize = 106;

static mut HWND_DIR: HWND = null_mut();
static mut HWND_LOG: HWND = null_mut();
static mut HFONT_NORMAL: HFONT = null_mut();
static mut HFONT_BOLD: HFONT = null_mut();

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe fn set_text(hwnd: HWND, text: &str) {
    let w = to_wide(text);
    SetWindowTextW(hwnd, w.as_ptr());
}

unsafe fn get_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..copied as usize])
}

unsafe fn show_msg(hwnd: HWND, title: &str, body: &str, flags: u32) {
    let tw = to_wide(title);
    let bw = to_wide(body);
    MessageBoxW(hwnd, bw.as_ptr(), tw.as_ptr(), flags);
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let hinst = GetModuleHandleW(null());
            let font_face = to_wide("Microsoft YaHei UI");
            HFONT_NORMAL = CreateFontW(
                -14, 0, 0, 0, 400, 0, 0, 0, DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32,
                CLEARTYPE_QUALITY as u32, (DEFAULT_PITCH | FF_DONTCARE) as u32,
                font_face.as_ptr(),
            );
            HFONT_BOLD = CreateFontW(
                -18, 0, 0, 0, 700, 0, 0, 0, DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32,
                CLEARTYPE_QUALITY as u32, (DEFAULT_PITCH | FF_DONTCARE) as u32,
                font_face.as_ptr(),
            );
            if HFONT_NORMAL.is_null() {
                HFONT_NORMAL = GetStockObject(DEFAULT_GUI_FONT as i32) as HFONT;
            }

            let payload = match load_embedded_payload() {
                Ok(p) => p,
                Err(e) => {
                    show_msg(hwnd, "译铸 LexiPatch 补丁引导器", &e, MB_OK | MB_ICONWARNING);
                    PostQuitMessage(0);
                    return 0;
                }
            };
            let m = &payload.manifest;

            let static_cls = to_wide("STATIC");
            let edit_cls = to_wide("EDIT");
            let btn_cls = to_wide("BUTTON");

            let header = format!("{} {}  —  独立汉化补丁", m.software_name, m.version);
            let h_title = CreateWindowExW(
                0, static_cls.as_ptr(), to_wide(&header).as_ptr(),
                WS_CHILD | WS_VISIBLE, 20, 16, 510, 26, hwnd, null_mut(), hinst, null(),
            );
            SendMessageW(h_title, WM_SETFONT, HFONT_BOLD as WPARAM, 1);

            let sub = format!(
                "项目：{}    包含文件：{} 个    制作：{}",
                m.project_name,
                m.files.len(),
                if m.author.is_empty() { "译铸 LexiPatch" } else { &m.author }
            );
            let h_sub = CreateWindowExW(
                0, static_cls.as_ptr(), to_wide(&sub).as_ptr(),
                WS_CHILD | WS_VISIBLE, 20, 48, 510, 20, hwnd, null_mut(), hinst, null(),
            );
            SendMessageW(h_sub, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);

            let h_lbl = CreateWindowExW(
                0, static_cls.as_ptr(), to_wide("目标软件安装目录：").as_ptr(),
                WS_CHILD | WS_VISIBLE, 20, 82, 200, 20, hwnd, null_mut(), hinst, null(),
            );
            SendMessageW(h_lbl, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);

            HWND_DIR = CreateWindowExW(
                0, edit_cls.as_ptr(), to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                20, 106, 410, 28, hwnd, ID_EDIT_DIR as _, hinst, null(),
            );
            SendMessageW(HWND_DIR, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);

            let h_browse = CreateWindowExW(
                0, btn_cls.as_ptr(), to_wide("浏览...").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                438, 106, 92, 28, hwnd, ID_BTN_BROWSE as _, hinst, null(),
            );
            SendMessageW(h_browse, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);

            HWND_LOG = CreateWindowExW(
                0, edit_cls.as_ptr(), to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32 | ES_AUTOVSCROLL as u32,
                20, 148, 510, 210, hwnd, ID_EDIT_LOG as _, hinst, null(),
            );
            SendMessageW(HWND_LOG, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);

            let h_install = CreateWindowExW(
                0, btn_cls.as_ptr(), to_wide("一键安装汉化").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
                20, 374, 150, 34, hwnd, ID_BTN_INSTALL as _, hinst, null(),
            );
            SendMessageW(h_install, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);
            SendMessageW(h_install, BM_SETSTYLE, BS_DEFPUSHBUTTON as WPARAM, 1);

            let h_restore = CreateWindowExW(
                0, btn_cls.as_ptr(), to_wide("还原原版备份").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                182, 374, 130, 34, hwnd, ID_BTN_RESTORE as _, hinst, null(),
            );
            SendMessageW(h_restore, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);

            let h_close = CreateWindowExW(
                0, btn_cls.as_ptr(), to_wide("退出").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                440, 374, 90, 34, hwnd, ID_BTN_CLOSE as _, hinst, null(),
            );
            SendMessageW(h_close, WM_SETFONT, HFONT_NORMAL as WPARAM, 1);

            // Auto detect directory & show initial info
            let mut init_log = Vec::new();
            if !m.note.trim().is_empty() {
                init_log.push(format!("补丁说明：{}\r\n", m.note.trim()));
            }
            if let Some(detected) = auto_detect_dir(m) {
                let s = detected.to_string_lossy().to_string();
                set_text(HWND_DIR, &s);
                init_log.push(format!("已自动定位到目标目录：{}", s));
            } else {
                init_log.push("未能自动定位软件安装目录，请点击右侧「浏览...」手动选择。".into());
            }
            init_log.push("\r\n待汉化文件列表：".into());
            for f in &m.files {
                init_log.push(format!("  • {} ({} 词条)", f.target_path, f.unit_count));
            }
            set_text(HWND_LOG, &init_log.join("\r\n"));
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            match id {
                ID_BTN_BROWSE => {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title("选择目标软件安装目录")
                        .pick_folder()
                    {
                        set_text(HWND_DIR, &folder.to_string_lossy());
                    }
                }
                ID_BTN_INSTALL => {
                    if let Ok(payload) = load_embedded_payload() {
                        let dir_str = get_text(HWND_DIR);
                        let target_dir = PathBuf::from(dir_str.trim());
                        if !target_dir.is_dir() {
                            show_msg(hwnd, "提示", "请先选择有效的软件安装目录。", MB_OK | MB_ICONWARNING);
                            return 0;
                        }
                        let matched = payload
                            .manifest
                            .files
                            .iter()
                            .filter(|f| target_dir.join(&f.target_path).exists())
                            .count();
                        if matched == 0 {
                            let tw = to_wide("目录确认");
                            let bw = to_wide("在所选目录中未检测到任何匹配的原始文件。\r\n是否仍要继续写入汉化文件？（如不确定，请点击「否」重新选择软件安装根目录）");
                            if MessageBoxW(hwnd, bw.as_ptr(), tw.as_ptr(), MB_YESNO | MB_ICONWARNING) != IDYES {
                                return 0;
                            }
                        }
                        match apply_patch(payload, &target_dir) {
                            Ok(log) => {
                                set_text(HWND_LOG, &log);
                                show_msg(hwnd, "汉化完成", "汉化补丁安装成功！原版文件已自动备份。", MB_OK | MB_ICONINFORMATION);
                            }
                            Err(err) => {
                                set_text(HWND_LOG, &format!("安装失败：\r\n{err}"));
                                show_msg(hwnd, "安装失败", &err, MB_OK | MB_ICONERROR);
                            }
                        }
                    }
                }
                ID_BTN_RESTORE => {
                    if let Ok(payload) = load_embedded_payload() {
                        let dir_str = get_text(HWND_DIR);
                        let target_dir = PathBuf::from(dir_str.trim());
                        match restore_backup(payload, &target_dir) {
                            Ok(log) => {
                                set_text(HWND_LOG, &log);
                                show_msg(hwnd, "还原完成", "已成功还原至原版状态！", MB_OK | MB_ICONINFORMATION);
                            }
                            Err(err) => {
                                set_text(HWND_LOG, &format!("还原失败：\r\n{err}"));
                                show_msg(hwnd, "还原失败", &err, MB_OK | MB_ICONERROR);
                            }
                        }
                    }
                }
                ID_BTN_CLOSE => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            if !HFONT_NORMAL.is_null() {
                DeleteObject(HFONT_NORMAL as _);
            }
            if !HFONT_BOLD.is_null() {
                DeleteObject(HFONT_BOLD as _);
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    // Support headless CLI mode: --install <dir> or --restore <dir>
    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 && (args[1] == "--install" || args[1] == "--restore") {
        let dir = PathBuf::from(&args[2]);
        if let Ok(payload) = load_embedded_payload() {
            let res = if args[1] == "--install" {
                apply_patch(payload, &dir)
            } else {
                restore_backup(payload, &dir)
            };
            if res.is_err() {
                std::process::exit(1);
            }
        } else {
            std::process::exit(2);
        }
        return;
    }

    unsafe {
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&icc);

        let hinst: HINSTANCE = GetModuleHandleW(null());
        let cls_name = to_wide("TransCorePatcherWindow");

        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: (windows_sys::Win32::Graphics::Gdi::COLOR_WINDOW + 1) as _,
            lpszMenuName: null(),
            lpszClassName: cls_name.as_ptr(),
        };
        RegisterClassW(&wc);

        let w = 566;
        let h = 460;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let x = (sx - w) / 2;
        let y = (sy - h) / 2;

        let title = match load_embedded_payload() {
            Ok(p) => format!("{} {} - 汉化补丁安装器", p.manifest.software_name, p.manifest.version),
            Err(_) => "译铸 LexiPatch 补丁引导器".to_string(),
        };

        let hwnd = CreateWindowExW(
            0,
            cls_name.as_ptr(),
            to_wide(&title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            x,
            y,
            w,
            h,
            null_mut(),
            null_mut(),
            hinst,
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        ShowWindow(hwnd, SW_SHOW);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
