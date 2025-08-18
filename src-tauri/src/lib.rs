// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serde::Deserialize;

// ===============================
// Front matter validation
// ===============================

/// Required front matter struct for Quarto
#[allow(dead_code)] // ignore dead_code warning
#[derive(Debug, Deserialize)]
struct FrontMatter {
    title: String,
    author: String,
    format: String,
    #[serde(rename = "embed-resources", default)]
    embed_resources: bool, // If missing, parsed as false → user-friendly error at validation stage
}

/// Markdown 문자열에서 YAML front matter 추출 (누수 없는 슬라이스 버전)
fn extract_yaml_front_matter(md: &str) -> Option<&str> {
    let mut lines = md.lines();

    // 첫 줄이 '---'인지 확인
    if !matches!(lines.next(), Some(line) if line.trim() == "---") {
        return None;
    }

    // 시작 인덱스: 첫 '---' 바로 뒤
    let start = md.find("---")? + 3;

    // 두 번째 '---'의 절대 인덱스 찾기 (start 이후)
    let rest = &md[start..];
    let end_rel = rest.find("\n---")?;
    let yaml = &rest[..end_rel + 1]; // 마지막 개행 포함
    Some(yaml)
}

/// 필수 필드/값 검증 함수
fn validate_markdown_header(md: &str) -> Result<FrontMatter, String> {
    let yaml = extract_yaml_front_matter(md)
        .ok_or_else(|| "Missing YAML front matter (--- ... ---) at the top of the file.".to_string())?;
    let fm: FrontMatter = serde_yaml::from_str(yaml)
        .map_err(|e| format!("Invalid YAML front matter: {}", e))?;

    // format 필수값 체크
    let allowed_formats = ["revealjs", "pptx", "beamer"];
    if !allowed_formats.iter().any(|&f| f == fm.format.trim()) {
        return Err(format!(
            "Unsupported format: {} (expected: revealjs, pptx, or beamer)",
            fm.format
        ));
    }
    // embed-resources 필수
    if !fm.embed_resources {
        return Err("`embed-resources: true` is required for self-contained slides.".to_string());
    }

    Ok(fm)
}

// ===============================
// Quarto detection (CLI)
// ===============================

#[tauri::command]
fn check_quarto_installed() -> Result<String, String> {
    use std::process::Command;

    // 번들 우선 → OS PATH 순
    let mac_path = "quarto/quarto-1.4.550/bin/quarto";
    let win_exe  = "quarto/quarto-1.4.550/bin/quarto.exe";
    let win_cmd  = "quarto.cmd"; // Windows PATH에서 자주 사용
    let candidates = [mac_path, win_exe, win_cmd, "quarto"];

    let mut tried = Vec::new();

    // dev_path: 환경변수 TAURI_DEV_PATH가 있으면 우선 사용, 없으면 기존 PATH에 주요 경로를 추가
    let dev_path = std::env::var("TAURI_DEV_PATH").ok().or_else(|| {
        let mut path = std::env::var("PATH").unwrap_or_default();
        let _home_cargo_bin = format!("{}/.cargo/bin", std::env::var("HOME").unwrap_or_default());
        let extra = vec![
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/Applications/quarto/bin",
            // 필요 시 home_cargo_bin.as_str()를 추가로 사용 가능
        ];
        for p in extra {
            if !path.split(':').any(|x| x == p) {
                path.push(':');
                path.push_str(p);
            }
        }
        Some(path)
    });

    for path in candidates.iter() {
        let mut cmd = Command::new(path);
        cmd.arg("--version");
        if let Some(dev_path) = dev_path.as_ref() {
            cmd.env("PATH", dev_path);
        }
        let output = cmd.output();
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

// ===============================
// Download / Save helpers
// ===============================

#[tauri::command]
fn download_rendered_html(html_path: String) -> Result<(String, String), String> {
    use base64::{engine::general_purpose, Engine as _};
    use std::fs;
    use std::path::Path; 

    let path = Path::new(&html_path);
    
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .map(|name| {
            // remove trailing _number before extension, e.g. ppt_1755325282888.pptx -> ppt.pptx
            let re = regex::Regex::new(r"^(.*?)(?:_\d+)?(\.[^.]+)$")
                .expect("Failed to compile regex for stripping trailing _number from filename");
            if let Some(caps) = re.captures(&name) {
                format!("{}{}", &caps[1], &caps[2])
            } else {
                name
            }
        });
        

    let filename = file_name.unwrap_or_else(|| "output.html".to_string());

    match fs::read(&path) {
        Ok(bytes) => {
            let encoded = general_purpose::STANDARD.encode(&bytes);
            Ok((filename, encoded))
        }
        Err(e) => Err(format!("HTML 파일 읽기 실패: {}", e)),
    }
}

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

    // Read HTML file
    let html_content = fs::read(&html_path)
        .map_err(|e| format!("HTML 파일 읽기 실패: {}", e))?;

    // File save dialog (callback → channel)
    let (tx, rx) = mpsc::channel();

    let mut dialog = window.dialog().file().set_file_name(&file_name);

    // If file_name has an extension, add that extension as a filter
    if let Some(ext) = std::path::Path::new(&file_name).extension().and_then(|s| s.to_str()) {
        dialog = dialog.add_filter(ext, &[ext]);
    }
    // Always add "All Files" filter
    dialog = dialog.add_filter("All Files", &["*"]);

    dialog.save_file(move |file_path| {
        let _ = tx.send(file_path);
    });

    // Receive result and save
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

// ===============================
// Render with DI (for testability)
// ===============================

use std::path::Path;

/// Quarto 실행 담당 인터페이스
trait QuartoRunner {
    fn render(&self, md_path: &Path, workdir: &Path, dev_path: Option<&str>) -> Result<(), String>;
}

/// 실제 러너: 외부 `quarto` 실행
struct RealRunner;

impl QuartoRunner for RealRunner {
    fn render(&self, md_path: &Path, workdir: &Path, dev_path: Option<&str>) -> Result<(), String> {
        use std::process::Command;

        let mac_path = "quarto/quarto-1.4.550/bin/quarto";
        let win_exe  = "quarto/quarto-1.4.550/bin/quarto.exe";
        let win_cmd  = "quarto.cmd";
        let candidates = [mac_path, win_exe, win_cmd, "quarto"];

        let mut last_err: Option<String> = None;

        for path in candidates.iter() {
            let mut cmd = Command::new(path);
            cmd.arg("render");
            cmd.arg(md_path);
            cmd.current_dir(workdir);
            if let Some(dev) = dev_path {
                cmd.env("PATH", dev);
            }
            let output = cmd.output();

            match output {
                Ok(out) if out.status.success() => return Ok(()),
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
}

/// 주 로직: runner 주입으로 테스트 가능하게 분리
fn render_with_runner<R: QuartoRunner>(
    runner: &R,
    md_content: String,
    orig_name: Option<String>,
) -> Result<String, String> {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    // 1) 입력 검증
    let _fm = validate_markdown_header(&md_content)?;

    // 2) 임시 파일명 구성
    let tmp_dir = env::temp_dir();
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let (stem, ext) = if let Some(name) = orig_name.clone() {
        let p = Path::new(&name);
        let s = p.file_stem().and_then(|s| s.to_str()).unwrap_or("temp_quarto");
        let e = p.extension().and_then(|e| e.to_str()).unwrap_or("md");
        (format!("{}_{}", s, ts), e.to_string())
    } else {
        (format!("temp_quarto_{}", ts), "md".to_string())
    };
    let ext = match ext.to_lowercase().as_str() {
        "md" | "qmd" => ext,
        _ => "md".to_string(),
    };



    let mut md_path: PathBuf = tmp_dir.clone();
    md_path.push(format!("{}.{}", stem, ext));

    // 3) 임시 파일 저장
    fs::write(&md_path, md_content).map_err(|e| format!("임시 파일 저장 실패: {}", e))?;

    // 4) dev_path 구성 (기존 로직 재사용)
    let dev_path = std::env::var("TAURI_DEV_PATH").ok().or_else(|| {
        let mut path = std::env::var("PATH").unwrap_or_default();
        let home_cargo_bin = format!("{}/.cargo/bin", std::env::var("HOME").unwrap_or_default());
        let extra = vec![
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/Applications/quarto/bin",
            home_cargo_bin.as_str(),
        ];
        for p in extra {
            if !path.split(':').any(|x| x == p) {
                path.push(':');
                path.push_str(p);
            }
        }
        Some(path)
    });

    // 5) Quarto 렌더 실행
    runner.render(&md_path, &tmp_dir, dev_path.as_deref())?;

    // 6) 결과 파일 확인 (html, pdf, pptx 순서로 탐색)
    let mut found = None;
    for ext in &["html", "pdf", "pptx"] {
        let candidate = md_path.with_extension(ext);
        if candidate.exists() {
            let abs = candidate.canonicalize().unwrap_or(candidate.clone());
            found = Some(abs.to_string_lossy().to_string());
            break;
        }
    }
    if let Some(path) = found {
        Ok(path)
    } else {
        Err(format!(
            "Quarto render는 성공했으나 결과 파일이 보이지 않습니다. 경로 prefix=`{}`",
            md_path.with_extension("").to_string_lossy()
        ))
    }
}

#[tauri::command]
fn render_quarto_file(md_content: String, orig_name: Option<String>) -> Result<String, String> {
    let runner = RealRunner;
    render_with_runner(&runner, md_content, orig_name)
}

// ===============================
// Tauri entry
// ===============================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            check_quarto_installed,
            render_quarto_file,
            download_rendered_html,
            save_html_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ===============================
// Tests
// ===============================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_header_parses() {
        let md = r#"---
title: "Intro"
author: "Alice"
format: revealjs
embed-resources: true
---
# Slide
"#;
        let fm = validate_markdown_header(md).expect("should parse");
        assert_eq!(fm.title, "Intro");     // title 필드 사용
        assert_eq!(fm.author, "Alice");    // author 필드 사용
        assert_eq!(fm.format, "revealjs");
        assert!(fm.embed_resources);
    }

    #[test]
    fn missing_front_matter_is_error() {
        let md = r#"# Slide
- content
"#;
        let err = validate_markdown_header(md).unwrap_err();
        assert!(err.contains("Missing YAML front matter"));
    }

    #[test]
    fn wrong_format_is_error() {
        let md = r#"---
title: "Intro"
author: "Alice"
format: invalidformat
embed-resources: true
---
# Slide
"#;
        let err = validate_markdown_header(md).unwrap_err();
        assert!(err.contains("Unsupported format"));
    }

    #[test]
    fn missing_embed_resources_is_error() {
        let md = r#"---
title: "Intro"
author: "Alice"
format: revealjs
---
# Slide
"#;
        let err = validate_markdown_header(md).unwrap_err();
        assert!(err.contains("`embed-resources: true`"));
    }

    #[test]
    fn malformed_yaml_is_error() {
        let md = r#"---
title: ["oops": "bad mapping"]
author: "Alice"
format: revealjs
embed-resources: true
---
# Slide
"#;
        let err = validate_markdown_header(md).unwrap_err();
        assert!(err.contains("Invalid YAML front matter"));
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    // 유효한 Markdown 샘플
    fn valid_md() -> String {
        r#"---
title: "Intro"
author: "Alice"
format: revealjs
embed-resources: true
---
# Slide 1
- item

# Slide 2
- item
"#.to_string()
    }

    // 성공 모킹: 렌더 호출 시 결과 HTML 파일을 작성
    struct MockRunnerOk;
    impl QuartoRunner for MockRunnerOk {
        fn render(&self, md_path: &Path, _workdir: &Path, _dev_path: Option<&str>) -> Result<(), String> {
            let html_path = md_path.with_extension("html");
            let html = r#"<!DOCTYPE html>
<html>
<head><title>Test Slides</title></head>
<body>
<div class="reveal">
  <div class="slides">
    <section><h1>Slide 1</h1></section>
    <section><h1>Slide 2</h1></section>
  </div>
</div>
</body>
</html>"#;
            fs::write(&html_path, html).map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    // 실패 모킹: 특정 에러 메시지 반환
    struct MockRunnerErr;
    impl QuartoRunner for MockRunnerErr {
        fn render(&self, _md_path: &Path, _workdir: &Path, _dev_path: Option<&str>) -> Result<(), String> {
            Err("mocked quarto error".to_string())
        }
    }

    // 미설치 모킹: Quarto가 아예 없는 상황
    struct MockRunnerNoQuarto;
    impl QuartoRunner for MockRunnerNoQuarto {
        fn render(&self, _md_path: &Path, _workdir: &Path, _dev_path: Option<&str>) -> Result<(), String> {
            Err("Quarto 실행 실패".to_string())
        }
    }

    #[test]
    fn render_writes_html_and_contains_section() {
        let md = valid_md();
        let runner = MockRunnerOk;

        let out_path = render_with_runner(&runner, md, Some("demo.md".to_string()))
            .expect("render_with_runner should succeed");

        // 파일이 생성되었는가?
        let html = fs::read_to_string(&out_path).expect("html must be readable");
        assert!(html.contains("<section>"), "HTML should contain slide <section> tags");
        assert!(html.contains("Slide 1"));
        assert!(html.contains("Slide 2"));
    }

    #[test]
    fn render_error_is_propagated() {
        let md = valid_md();
        let runner = MockRunnerErr;

        let err = render_with_runner(&runner, md, Some("demo.md".to_string()))
            .unwrap_err();

        assert!(err.contains("mocked quarto error"));
    }

    #[test]
    fn no_quarto_installed_returns_clear_error() {
        let md = valid_md();
        let runner = MockRunnerNoQuarto;

        let err = render_with_runner(&runner, md, Some("demo.md".to_string()))
            .unwrap_err();

        assert!(
            err.contains("Quarto 실행 실패"),
            "Error message should clearly indicate Quarto is missing, got: {}",
            err
        );
    }
}
