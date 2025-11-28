use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, AtomicI32, AtomicBool, Ordering};
use std::thread;
use minhook_sys::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowsHookExW, CallNextHookEx, GetMessageW,
    WH_MOUSE_LL, MSLLHOOKSTRUCT, HHOOK, MSG,
};
use windows::Win32::Foundation::{WPARAM, LPARAM, LRESULT};

use crate::config_manager::{ZoomConfig, get_config, init_config};

static ORIGINAL_RENDER_LEVEL: AtomicUsize = AtomicUsize::new(0);
static MOUSE_HOOK: AtomicUsize = AtomicUsize::new(0);
static SCROLL_DELTA: AtomicI32 = AtomicI32::new(0);
static ZOOM_KEY_PRESSED: AtomicBool = AtomicBool::new(false);

const WM_MOUSEWHEEL: u32 = 0x020A;


const RENDER_LEVEL_SIG: &[u8] = &[
    0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x00, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0xA8, 0x00, 0x00, 0x00, 0x00, 0x48, 0x81, 0xEC, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x29, 0x70, 0x00, 0x0F, 0x29, 0x78, 0x00, 0x44, 0x0F, 0x29, 0x40, 0x00, 0x44, 0x0F, 0x29, 0x48, 0x00, 0x48, 0x8B, 0x05, 0x00, 0x00, 0x00, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x85, 0x00, 0x00, 0x00, 0x00, 0x4D, 0x8B, 0xE8, 0x4C, 0x8B, 0xE2, 0x4C, 0x8B, 0xF9
];

const RENDER_LEVEL_MASK: &[u8] = &[
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
];

#[repr(C)]
struct LevelRenderer {
    padding: [u8; 0x3F0],
    player: *mut LevelRendererPlayer,
}

#[repr(C)]
struct LevelRendererPlayer {
    padding_x: [u8; 0xF80],
    fov_x: f32,
    padding_y: [u8; 0xF94 - 0xF80 - 4],
    fov_y: f32,
}

static mut ZOOM_MODIFIER: f32 = 1.0;
static mut CURRENT_ZOOM_LEVEL: f32 = 10.0;

/// マウスホイールのフックプロシージャ
unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_MOUSEWHEEL {
        let mouse_struct = lparam.0 as *const MSLLHOOKSTRUCT;
        if !mouse_struct.is_null() {
            // mouseDataの上位ワードにホイールデルタが含まれている
            let wheel_delta = ((*mouse_struct).mouseData >> 16) as i16 as i32;
            // デルタを蓄積（120単位で1ノッチ）
            SCROLL_DELTA.fetch_add(wheel_delta, Ordering::Relaxed);
        }
    }
    
    let hook = HHOOK(MOUSE_HOOK.load(Ordering::Relaxed) as isize);
    CallNextHookEx(hook, code, wparam, lparam)
}

/// マウスフック用のメッセージループスレッド
fn start_mouse_hook_thread() {
    thread::spawn(|| {
        unsafe {
            // マウスホイールフックをインストール
            if let Ok(hook) = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0) {
                MOUSE_HOOK.store(hook.0 as usize, Ordering::Relaxed);
                
                // メッセージループ（フックが動作するために必要）
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    // メッセージを処理（特に何もしない）
                }
            }
        }
    });
}

unsafe extern "C" fn detour_render_level(level_renderer: *mut LevelRenderer, screen_context: *mut c_void, unk: *mut c_void) {
    let original_addr = ORIGINAL_RENDER_LEVEL.load(Ordering::Relaxed);
    if original_addr != 0 {
        let original: extern "C" fn(*mut LevelRenderer, *mut c_void, *mut c_void) = std::mem::transmute(original_addr);
        
        original(level_renderer, screen_context, unk);

        if !level_renderer.is_null() {
             let player = (*level_renderer).player;
             if !player.is_null() {
                 // 設定を取得（ファイルが更新されていたら自動で再読み込み）
                 let config = get_config();
                 
                 // 保存されたズーム倍率を使用
                 CURRENT_ZOOM_LEVEL = config.zoom_level;
                 
                 let is_zoom_key_pressed = GetAsyncKeyState(config.zoom_key) & 0x8000u16 as i16 != 0;
                 
                 // ズームキーの状態を更新（フック用）
                 ZOOM_KEY_PRESSED.store(is_zoom_key_pressed, Ordering::Relaxed);
                 
                 // ズーム中にマウスホイールで倍率を調整（設定で有効な場合のみ）
                 if is_zoom_key_pressed && config.scroll_adjustment {
                     let scroll_delta = SCROLL_DELTA.swap(0, Ordering::Relaxed);
                     if scroll_delta != 0 {
                         // 120単位でscroll_step変更（スクロール1ノッチ = 120）
                         let zoom_change = (scroll_delta as f32 / 120.0) * config.scroll_step;
                         CURRENT_ZOOM_LEVEL = (CURRENT_ZOOM_LEVEL + zoom_change).clamp(1.0, 50.0);
                         save_zoom_level(CURRENT_ZOOM_LEVEL, &config);
                     }
                 } else if !is_zoom_key_pressed {
                     // ズームキーが押されていない時はスクロールデルタをクリア
                     SCROLL_DELTA.store(0, Ordering::Relaxed);
                 }
                 
                 let target = if is_zoom_key_pressed { CURRENT_ZOOM_LEVEL } else { 1.0 };
                 
                 if config.smooth_animation {
                     // スムーズアニメーション有効時: 補間で滑らかにズーム
                     ZOOM_MODIFIER = ZOOM_MODIFIER + (target - ZOOM_MODIFIER) * config.animation_speed;
                 } else {
                     // スムーズアニメーション無効時: 即座にズーム
                     ZOOM_MODIFIER = target;
                 }
                 
                 (*player).fov_x *= ZOOM_MODIFIER;
                 (*player).fov_y *= ZOOM_MODIFIER;
             }
        }
    }
}

/// ズーム倍率を設定ファイルに保存
fn save_zoom_level(zoom_level: f32, current_config: &ZoomConfig) {
    let mut config = current_config.clone();
    config.zoom_level = zoom_level;
    let _ = config.save();
}

pub unsafe fn initialize() {
    // 設定を初期化
    init_config();
    
    // マウスホイールフックを別スレッドで開始
    start_mouse_hook_thread();
    
    let base = windows::Win32::System::LibraryLoader::GetModuleHandleA(None).unwrap();
    
    let dos_header = base.0 as *const windows::Win32::System::SystemServices::IMAGE_DOS_HEADER;
    let nt_headers = (base.0 as usize + (*dos_header).e_lfanew as usize) as *const windows::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64;
    let size_of_image = (*nt_headers).OptionalHeader.SizeOfImage as usize;
    
    let memory_slice = std::slice::from_raw_parts(base.0 as *const u8, size_of_image);
    
    if let Some(offset) = find_pattern(memory_slice, RENDER_LEVEL_SIG, RENDER_LEVEL_MASK) {
        let target_addr = (base.0 as usize + offset) as *mut c_void;
        

        
        let mut original: *mut c_void = std::ptr::null_mut();
        if MH_CreateHook(target_addr, detour_render_level as *mut c_void, &mut original) == MH_OK {
            ORIGINAL_RENDER_LEVEL.store(original as usize, Ordering::Relaxed);
            MH_EnableHook(target_addr);
        }
    }
}

fn find_pattern(data: &[u8], pattern: &[u8], mask: &[u8]) -> Option<usize> {
    if pattern.len() != mask.len() {
        return None;
    }
    
    for i in 0..data.len() - pattern.len() {
        let mut found = true;
        for j in 0..pattern.len() {
            if mask[j] == 0xFF && data[i + j] != pattern[j] {
                found = false;
                break;
            }
        }
        if found {
            return Some(i);
        }
    }
    None
}
