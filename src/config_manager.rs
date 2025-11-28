use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// YAML設定ファイル用の構造体
#[derive(Serialize, Deserialize, Clone)]
pub struct ZoomConfigYaml {
    /// ズームをトリガーするキー名 (例: "C", "Z", "F5")
    #[serde(default = "default_zoom_key")]
    pub zoom_key: String,
    /// スムーズズームアニメーションを有効にするかどうか
    #[serde(default = "default_smooth_animation")]
    pub smooth_animation: bool,
    /// アニメーションの速度 (0.01 ~ 1.0, 高いほど速い)
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f32,
    /// ズーム倍率 (1.0 ~ 50.0)
    #[serde(default = "default_zoom_level")]
    pub zoom_level: f32,
}

fn default_zoom_key() -> String { "C".to_string() }
fn default_smooth_animation() -> bool { true }
fn default_animation_speed() -> f32 { 0.1 }
fn default_zoom_level() -> f32 { 10.0 }

impl Default for ZoomConfigYaml {
    fn default() -> Self {
        Self {
            zoom_key: default_zoom_key(),
            smooth_animation: default_smooth_animation(),
            animation_speed: default_animation_speed(),
            zoom_level: default_zoom_level(),
        }
    }
}

/// Zoom機能の設定を管理する構造体 (内部使用)
#[derive(Clone)]
pub struct ZoomConfig {
    /// ズームをトリガーするキーコード (デフォルト: VK_C = 0x43)
    pub zoom_key: i32,
    /// スムーズズームアニメーションを有効にするかどうか
    pub smooth_animation: bool,
    /// アニメーションの速度 (0.0 ~ 1.0, 高いほど速い)
    pub animation_speed: f32,
    /// ズーム倍率
    pub zoom_level: f32,
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            zoom_key: VK_C.0 as i32,
            smooth_animation: true,
            animation_speed: 0.1,
            zoom_level: 10.0,
        }
    }
}

impl From<ZoomConfigYaml> for ZoomConfig {
    fn from(yaml: ZoomConfigYaml) -> Self {
        Self {
            zoom_key: Self::parse_key(&yaml.zoom_key),
            smooth_animation: yaml.smooth_animation,
            animation_speed: yaml.animation_speed.clamp(0.01, 1.0),
            zoom_level: yaml.zoom_level.clamp(1.0, 50.0),
        }
    }
}

impl ZoomConfig {
    /// 設定ファイルのパスを取得
    fn config_path() -> PathBuf {
        let mut path = std::env::current_exe().unwrap_or_default();
        path.pop();
        path.push("zoom_config.yml");
        path
    }

    /// 設定をファイルから読み込む
    pub fn load() -> Self {
        let path = Self::config_path();
        
        if let Ok(content) = fs::read_to_string(&path) {
            match serde_yaml::from_str::<ZoomConfigYaml>(&content) {
                Ok(yaml_config) => yaml_config.into(),
                Err(_) => {
                    // パースエラーの場合はデフォルト設定を使用
                    let config = ZoomConfig::default();
                    let _ = config.save();
                    config
                }
            }
        } else {
            // 設定ファイルが存在しない場合、デフォルト設定で作成
            let config = ZoomConfig::default();
            let _ = config.save();
            config
        }
    }

    /// 設定をファイルに保存
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        
        let yaml_config = ZoomConfigYaml {
            zoom_key: Self::key_to_string(self.zoom_key),
            smooth_animation: self.smooth_animation,
            animation_speed: self.animation_speed,
            zoom_level: self.zoom_level,
        };
        
        let header = r#"# Zoom Configuration File / ズーム設定ファイル
#
# zoom_key: ズームをトリガーするキー
#   使用可能なキー名: A-Z, 0-9, F1-F12, CTRL, SHIFT, ALT, SPACE, TAB, ENTER, ESC
#   例: "C", "Z", "F5", "CTRL"
#
# smooth_animation: スムーズズームアニメーション
#   true: 滑らかなズームアニメーション
#   false: 即座にズーム
#
# animation_speed: アニメーション速度 (0.01 ~ 1.0)
#   値が大きいほど速い
#
# zoom_level: ズーム倍率 (1.0 ~ 50.0)

"#;
        
        let yaml_content = serde_yaml::to_string(&yaml_config)
            .unwrap_or_else(|_| "zoom_key: C\nsmooth_animation: true\nanimation_speed: 0.1\nzoom_level: 10.0\n".to_string());
        
        fs::write(path, format!("{}{}", header, yaml_content))
    }

    /// キー名を仮想キーコードに変換
    fn parse_key(value: &str) -> i32 {
        let value = value.to_uppercase();
        
        // 16進数キーコードの処理
        if value.starts_with("0X") {
            if let Ok(code) = i32::from_str_radix(&value[2..], 16) {
                return code;
            }
        }
        
        // 数値キーコードの処理
        if let Ok(code) = value.parse::<i32>() {
            return code;
        }
        
        // キー名からキーコードへの変換
        match value.as_str() {
            // アルファベットキー
            "A" => VK_A.0 as i32,
            "B" => VK_B.0 as i32,
            "C" => VK_C.0 as i32,
            "D" => VK_D.0 as i32,
            "E" => VK_E.0 as i32,
            "F" => VK_F.0 as i32,
            "G" => VK_G.0 as i32,
            "H" => VK_H.0 as i32,
            "I" => VK_I.0 as i32,
            "J" => VK_J.0 as i32,
            "K" => VK_K.0 as i32,
            "L" => VK_L.0 as i32,
            "M" => VK_M.0 as i32,
            "N" => VK_N.0 as i32,
            "O" => VK_O.0 as i32,
            "P" => VK_P.0 as i32,
            "Q" => VK_Q.0 as i32,
            "R" => VK_R.0 as i32,
            "S" => VK_S.0 as i32,
            "T" => VK_T.0 as i32,
            "U" => VK_U.0 as i32,
            "V" => VK_V.0 as i32,
            "W" => VK_W.0 as i32,
            "X" => VK_X.0 as i32,
            "Y" => VK_Y.0 as i32,
            "Z" => VK_Z.0 as i32,
            
            // ファンクションキー
            "F1" => VK_F1.0 as i32,
            "F2" => VK_F2.0 as i32,
            "F3" => VK_F3.0 as i32,
            "F4" => VK_F4.0 as i32,
            "F5" => VK_F5.0 as i32,
            "F6" => VK_F6.0 as i32,
            "F7" => VK_F7.0 as i32,
            "F8" => VK_F8.0 as i32,
            "F9" => VK_F9.0 as i32,
            "F10" => VK_F10.0 as i32,
            "F11" => VK_F11.0 as i32,
            "F12" => VK_F12.0 as i32,
            
            // 修飾キー
            "CTRL" | "CONTROL" => VK_CONTROL.0 as i32,
            "SHIFT" => VK_SHIFT.0 as i32,
            "ALT" | "MENU" => VK_MENU.0 as i32,
            
            // その他のキー
            "SPACE" => VK_SPACE.0 as i32,
            "TAB" => VK_TAB.0 as i32,
            "ENTER" | "RETURN" => VK_RETURN.0 as i32,
            "ESCAPE" | "ESC" => VK_ESCAPE.0 as i32,
            "BACKSPACE" | "BACK" => VK_BACK.0 as i32,
            
            // 数字キー
            "0" => 0x30,
            "1" => 0x31,
            "2" => 0x32,
            "3" => 0x33,
            "4" => 0x34,
            "5" => 0x35,
            "6" => 0x36,
            "7" => 0x37,
            "8" => 0x38,
            "9" => 0x39,
            
            // デフォルトはCキー
            _ => VK_C.0 as i32,
        }
    }

    /// キーコードをキー名に変換
    fn key_to_string(key_code: i32) -> String {
        match key_code {
            x if x == VK_A.0 as i32 => "A".to_string(),
            x if x == VK_B.0 as i32 => "B".to_string(),
            x if x == VK_C.0 as i32 => "C".to_string(),
            x if x == VK_D.0 as i32 => "D".to_string(),
            x if x == VK_E.0 as i32 => "E".to_string(),
            x if x == VK_F.0 as i32 => "F".to_string(),
            x if x == VK_G.0 as i32 => "G".to_string(),
            x if x == VK_H.0 as i32 => "H".to_string(),
            x if x == VK_I.0 as i32 => "I".to_string(),
            x if x == VK_J.0 as i32 => "J".to_string(),
            x if x == VK_K.0 as i32 => "K".to_string(),
            x if x == VK_L.0 as i32 => "L".to_string(),
            x if x == VK_M.0 as i32 => "M".to_string(),
            x if x == VK_N.0 as i32 => "N".to_string(),
            x if x == VK_O.0 as i32 => "O".to_string(),
            x if x == VK_P.0 as i32 => "P".to_string(),
            x if x == VK_Q.0 as i32 => "Q".to_string(),
            x if x == VK_R.0 as i32 => "R".to_string(),
            x if x == VK_S.0 as i32 => "S".to_string(),
            x if x == VK_T.0 as i32 => "T".to_string(),
            x if x == VK_U.0 as i32 => "U".to_string(),
            x if x == VK_V.0 as i32 => "V".to_string(),
            x if x == VK_W.0 as i32 => "W".to_string(),
            x if x == VK_X.0 as i32 => "X".to_string(),
            x if x == VK_Y.0 as i32 => "Y".to_string(),
            x if x == VK_Z.0 as i32 => "Z".to_string(),
            x if x == VK_F1.0 as i32 => "F1".to_string(),
            x if x == VK_F2.0 as i32 => "F2".to_string(),
            x if x == VK_F3.0 as i32 => "F3".to_string(),
            x if x == VK_F4.0 as i32 => "F4".to_string(),
            x if x == VK_F5.0 as i32 => "F5".to_string(),
            x if x == VK_F6.0 as i32 => "F6".to_string(),
            x if x == VK_F7.0 as i32 => "F7".to_string(),
            x if x == VK_F8.0 as i32 => "F8".to_string(),
            x if x == VK_F9.0 as i32 => "F9".to_string(),
            x if x == VK_F10.0 as i32 => "F10".to_string(),
            x if x == VK_F11.0 as i32 => "F11".to_string(),
            x if x == VK_F12.0 as i32 => "F12".to_string(),
            x if x == VK_CONTROL.0 as i32 => "CTRL".to_string(),
            x if x == VK_SHIFT.0 as i32 => "SHIFT".to_string(),
            x if x == VK_MENU.0 as i32 => "ALT".to_string(),
            x if x == VK_SPACE.0 as i32 => "SPACE".to_string(),
            x if x == VK_TAB.0 as i32 => "TAB".to_string(),
            x if x == VK_RETURN.0 as i32 => "ENTER".to_string(),
            x if x == VK_ESCAPE.0 as i32 => "ESC".to_string(),
            x if x >= 0x30 && x <= 0x39 => ((x - 0x30) as u8 + b'0').to_string(),
            _ => format!("0x{:02X}", key_code),
        }
    }
}

// グローバル設定インスタンス
static CONFIG: RwLock<Option<ZoomConfig>> = RwLock::new(None);
static LAST_MODIFIED: RwLock<Option<SystemTime>> = RwLock::new(None);

/// 設定ファイルの更新時刻を取得
fn get_file_modified_time() -> Option<SystemTime> {
    let path = ZoomConfig::config_path();
    fs::metadata(&path).ok()?.modified().ok()
}

/// 設定を初期化して読み込む
pub fn init_config() -> ZoomConfig {
    let config = ZoomConfig::load();
    if let Ok(mut guard) = CONFIG.write() {
        *guard = Some(config.clone());
    }
    if let Ok(mut guard) = LAST_MODIFIED.write() {
        *guard = get_file_modified_time();
    }
    config
}

/// 現在の設定を取得（ファイルが更新されていたら再読み込み）
pub fn get_config() -> ZoomConfig {
    // ファイルの更新時刻をチェック
    let current_modified = get_file_modified_time();
    let needs_reload = if let Ok(guard) = LAST_MODIFIED.read() {
        match (&*guard, &current_modified) {
            (Some(last), Some(current)) => current > last,
            (None, Some(_)) => true,
            _ => false,
        }
    } else {
        false
    };
    
    if needs_reload {
        return reload_config();
    }
    
    if let Ok(guard) = CONFIG.read() {
        if let Some(ref config) = *guard {
            return config.clone();
        }
    }
    
    // 設定がまだ読み込まれていない場合は初期化
    init_config()
}

/// 設定を再読み込み
pub fn reload_config() -> ZoomConfig {
    init_config()
}
