// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn check_quarto_installed() -> Result<String, String> {
    use std::process::Command;

    // 번들 우선 → OS PATH 순
    let mac_path = "quarto/quarto-1.4.550/bin/quarto";
    let win_exe  = "quarto/quarto-1.4.550/bin/quarto.exe";
    let win_cmd  = "quarto.cmd"; // Windows PATH에서 자주 사용
    let candidates = [mac_path, win_exe, win_cmd, "quarto"];

    let mut tried = Vec::new();

    for path in candidates.iter() {
        let output = Command::new(path).arg("--version").output();
        match output {
            Ok(out) if out.status.success() => {
                let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return Ok(ver);
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                tried.push(format!("{} -> exit={}, stderr={}", path, out.status, err));
            }
            Err(e) => {
                tried.push(format!("{} -> err={}", path, e));
            }
        }
    }

    Err(format!(
        "Quarto를 찾을 수 없습니다. 시도한 경로/명령: {}",
        tried.join(" | ")
    ))
}

// Quarto render: 업로드된 내용을 임시 파일로 저장 후 렌더 → 생성된 HTML 절대 경로 반환
#[tauri::command]
fn render_quarto_file(md_content: String, orig_name: Option<String>) -> Result<String, String> {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    // 임시 디렉터리
    let tmp_dir = env::temp_dir();

    // 유니크 파일명 (원본 스템 + 타임스탬프)
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let (stem, ext) = if let Some(name) = orig_name.clone() {
        let p = Path::new(&name);
        let s = p.file_stem().and_then(|s| s.to_str()).unwrap_or("temp_quarto");
        let e = p.extension().and_then(|e| e.to_str()).unwrap_or("md");
        (format!("{}_{}", s, ts), e.to_string())
    } else {
        (format!("temp_quarto_{}", ts), "md".to_string())
    };

    // md/qmd 이외 확장자는 md로 강제
    let ext = match ext.to_lowercase().as_str() {
        "md" | "qmd" => ext,
        _ => "md".to_string(),
    };

    let mut md_path: PathBuf = tmp_dir.clone();
    md_path.push(format!("{}.{}", stem, ext));

    // 파일 저장
    if let Err(e) = fs::write(&md_path, md_content) {
        return Err(format!("임시 파일 저장 실패: {}", e));
    }

    // 후보 바이너리
    let mac_path = "quarto/quarto-1.4.550/bin/quarto";
    let win_exe  = "quarto/quarto-1.4.550/bin/quarto.exe";
    let win_cmd  = "quarto.cmd";
    let candidates = [mac_path, win_exe, win_cmd, "quarto"];

    let html_path = md_path.with_extension("html");
    let mut last_err: Option<String> = None;

    for path in candidates.iter() {
        // 작업 디렉터리를 임시 폴더로 고정 (상대 출력 경로 문제 방지)
        let output = Command::new(path)
            .arg("render")
            .arg(&md_path)
            // .arg("--to").arg("html")
            .current_dir(&tmp_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                if html_path.exists() {
                    // 절대 경로 문자열로 반환
                    let abs = html_path.canonicalize().unwrap_or(html_path.clone());
                    return Ok(abs.to_string_lossy().to_string());
                } else {
                    let so = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let se = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    return Err(format!(
                        "Quarto render 성공 코드이나 HTML이 보이지 않습니다. stdout=`{}`, stderr=`{}`, 경로=`{}`",
                        so, se, html_path.to_string_lossy()
                    ));
                }
            }
            Ok(out) => {
                let se = String::from_utf8_lossy(&out.stderr).trim().to_string();
                last_err = Some(format!("{}: exit={}, stderr={}", path, out.status, se));
            }
            Err(e) => {
                last_err = Some(format!("{}: 실행 오류 {}", path, e));
            }
        }
    }

    Err(last_err.unwrap_or_else(|| "Quarto 실행 실패".to_string()))
}

// HTML 파일을 base64로 반환 (JS에서 atob/Uint8Array로 디코드해 저장)
#[tauri::command]
fn download_rendered_html(html_path: String) -> Result<(String, String), String> {
    use base64::{engine::general_purpose, Engine as _};
    use std::fs;
    use std::path::Path;

    let path = Path::new(&html_path);
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "output.html".to_string());

    match fs::read(&path) {
        Ok(bytes) => {
            let encoded = general_purpose::STANDARD.encode(&bytes);
            Ok((file_name, encoded))
        }
        Err(e) => Err(format!("HTML 파일 읽기 실패: {}", e)),
    }
}

// 네이티브 저장 대화상자 사용 (선택)
#[tauri::command]
async fn save_html_file(
    window: tauri::Window,
    html_path: String,
    default_name: Option<String>
) -> Result<String, String> {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use tauri_plugin_dialog::{DialogExt, FilePath};

    let file_name = default_name.unwrap_or_else(|| "output.html".to_string());

    // HTML 읽기
    let html_content = fs::read(&html_path)
        .map_err(|e| format!("HTML 파일 읽기 실패: {}", e))?;

    // 파일 저장 대화상자 (콜백 → 채널)
    let (tx, rx) = mpsc::channel();
    window
        .dialog()
        .file()
        .set_file_name(&file_name)
        .add_filter("HTML", &["html"])
        .add_filter("All Files", &["*"])
        .save_file(move |file_path| {
            let _ = tx.send(file_path);
        });

    // 결과 수신 및 변환
    let save_path: Option<PathBuf> = rx
        .recv()
        .map_err(|_| "파일 선택 대화상자 오류".to_string())?
        .map(|fp: FilePath| fp.into_path().ok())
        .flatten();

    if let Some(path) = save_path {
        fs::write(&path, html_content)
            .map_err(|e| format!("파일 저장 실패: {}", e))?;
        Ok(format!("저장 완료: {}", path.to_string_lossy()))
    } else {
        Err("저장이 취소되었습니다.".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            check_quarto_installed,
            render_quarto_file,
            download_rendered_html,
            save_html_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
