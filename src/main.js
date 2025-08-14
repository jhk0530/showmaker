const { invoke } = window.__TAURI__.core;

// Tauri dialog와 fs API
let saveDialog = null;
let writeBinaryFile = null;

// Tauri API 동적 로드
window.addEventListener("DOMContentLoaded", async () => {
  if (window.__TAURI__) {
    try {
      // Tauri v2에서는 import 형태로 API를 가져와야 함
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { writeFile } = await import("@tauri-apps/plugin-fs");
      saveDialog = save;
      writeBinaryFile = writeFile;
      console.log("Tauri API 로드 성공");
    } catch (e) {
      console.log("Tauri API import 실패, fallback 사용:", e);
    }
  }
});

let greetInputEl;
let greetMsgEl;

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsgEl.textContent = await invoke("greet", { name: greetInputEl.value });
}

window.addEventListener("DOMContentLoaded", () => {
  // greet-form이 있을 때만 greet 관련 코드 실행
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  const greetForm = document.querySelector("#greet-form");
  if (greetForm && greetInputEl && greetMsgEl) {
    greetForm.addEventListener("submit", (e) => {
      e.preventDefault();
      greet();
    });
  }

  // 마크다운 업로드 및 Quarto Render 버튼
  const upload = document.getElementById("md-upload");
  const renderBtn = document.getElementById("quarto-render-btn");
  const renderMsg = document.getElementById("quarto-render-msg");
  const downloadBtn = document.getElementById("quarto-download-btn");
  const downloadNameInput = document.getElementById("quarto-download-name");
  let lastMdContent = null;
  let lastMdName = null;
  let lastHtmlPath = null;
  if (upload) {
    upload.addEventListener("change", (e) => {
      const file = e.target.files[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = function (ev) {
        const mdText = ev.target.result;
        // 미리보기 없이 업로드 내용만 저장
        lastMdContent = mdText;
        lastMdName = file.name;
      };
      reader.readAsText(file);
      if (renderBtn) renderBtn.style.display = "inline-flex";
    });
  }
  if (renderBtn) {
    renderBtn.addEventListener("click", async () => {
      if (!lastMdContent) {
        renderMsg.textContent = "먼저 마크다운 파일을 업로드하세요.";
        return;
      }
      renderMsg.textContent = "Quarto render 실행 중...";
      if (downloadBtn) downloadBtn.style.display = "none";
      if (downloadNameInput) downloadNameInput.style.display = "none";
      try {
        const htmlPath = await invoke("render_quarto_file", {
          mdContent: lastMdContent,
          origName: lastMdName,
        });
        renderMsg.textContent = "Quarto render 성공!";
        lastHtmlPath = htmlPath;
        if (downloadBtn) downloadBtn.style.display = "inline-block";
        if (downloadNameInput) {
          // 기본 파일명: 원본 마크다운 파일명에서 .md → .html
          let base = lastMdName ? lastMdName.replace(/\.[^.]+$/, "") : "output";
          downloadNameInput.value = base + ".html";
          downloadNameInput.style.display = "inline-block";
        }
      } catch (e) {
        renderMsg.textContent = "Quarto render 실패: " + e;
        if (downloadBtn) downloadBtn.style.display = "none";
        if (downloadNameInput) downloadNameInput.style.display = "none";
      }
    });
  }

  if (downloadBtn) {
    downloadBtn.addEventListener("click", async () => {
      if (!lastHtmlPath) {
        renderMsg.textContent = "HTML 경로가 없습니다.";
        console.log("[다운로드] lastHtmlPath 없음");
        return;
      }
      downloadBtn.textContent = "다운로드 준비 중...";
      try {
        const [fileName, base64] = await invoke("download_rendered_html", {
          htmlPath: lastHtmlPath,
        });
        console.log("[다운로드] fileName:", fileName);
        console.log("[다운로드] base64 length:", base64 ? base64.length : 0);
        let saveName =
          downloadNameInput && downloadNameInput.value
            ? downloadNameInput.value
            : fileName;

        // Tauri 네이티브 파일 저장 대화상자 사용
        try {
          const result = await invoke("save_html_file", {
            htmlPath: lastHtmlPath,
            defaultName: saveName,
          });
          renderMsg.textContent = result;
        } catch (saveError) {
          console.error("Tauri 저장 실패:", saveError);
          // fallback으로 브라우저 다운로드 사용
          const link = document.createElement("a");
          link.href = "data:text/html;base64," + base64;
          link.download = saveName;
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
          renderMsg.textContent = "HTML 다운로드 완료 (fallback)";
        }
        downloadBtn.textContent = "HTML 다운로드";
      } catch (e) {
        renderMsg.textContent = "HTML 다운로드 실패: " + e;
        downloadBtn.textContent = "HTML 다운로드";
        console.error("[다운로드] 에러:", e);
      }
    });
  }

  // Quarto 설치 상태 indicator는 항상 동작
  const indicator = document.getElementById("quarto-indicator");
  const statusText = document.getElementById("quarto-status-text");
  const officialBtn = document.getElementById("quarto-official-btn");
  if (indicator && statusText && typeof invoke === "function") {
    invoke("check_quarto_installed")
      .then((ver) => {
        indicator.style.background = "#4caf50";
        statusText.textContent = `설치됨 (${ver})`;
        if (officialBtn) officialBtn.style.display = "none";
      })
      .catch((err) => {
        indicator.style.background = "#f44336";
        statusText.textContent = "설치되지 않음";
        if (officialBtn) officialBtn.style.display = "inline-block";
      });
    if (officialBtn) {
      officialBtn.onclick = () => {
        window.open("https://quarto.org/docs/get-started/", "_blank");
      };
    }
  }
});
