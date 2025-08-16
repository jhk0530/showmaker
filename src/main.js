const { invoke } = window.__TAURI__.core;

window.addEventListener("DOMContentLoaded", () => {
  // 마크다운 업로드 및 Quarto Render 버튼
  const upload = document.getElementById("md-upload");
  const renderBtn = document.getElementById("quarto-render-btn");
  const downloadBtn = document.getElementById("quarto-download-btn");
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
        console.log("먼저 마크다운 파일을 업로드하세요.");
        return;
      }
      if (downloadBtn) downloadBtn.style.display = "none";
      try {
        const htmlPath = await invoke("render_quarto_file", {
          mdContent: lastMdContent,
          origName: lastMdName,
        });
        console.log("Quarto render 성공!");
        lastHtmlPath = htmlPath;
        if (downloadBtn) downloadBtn.style.display = "inline-flex";
      } catch (e) {
        console.log("Quarto render 실패:", e);
        if (downloadBtn) downloadBtn.style.display = "none";
      }
    });
  }

  if (downloadBtn) {
    downloadBtn.addEventListener("click", async () => {
      try {
        const [fileName, base64] = await invoke("download_rendered_html", {
          htmlPath: lastHtmlPath,
        });
        console.log("htmlPath: ", lastHtmlPath);
        console.log("[다운로드] fileName:", fileName);
        console.log("[다운로드] base64 length:", base64 ? base64.length : 0);
        console.log("lastMdname: ", lastMdName);

        // fileName's extension
        let ext = fileName.split(".").pop();
        console.log("ext: ", ext);

        let saveName = fileName;
        console.log("saveName: ", saveName);

        // Tauri 네이티브 파일 저장 대화상자 사용
        try {
          const result = await invoke("save_html_file", {
            htmlPath: lastHtmlPath,
            defaultName: saveName,
          });
          console.log(result);
        } catch (saveError) {
          console.error("Tauri 저장 실패:", saveError);
          // fallback으로 브라우저 다운로드 사용
          const link = document.createElement("a");
          link.href = "data:text/html;base64," + base64;
          link.download = saveName;
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
          console.log("HTML 다운로드 완료 (fallback)");
        }
      } catch (e) {
        console.log("HTML 다운로드 실패:", e);
        downloadBtn.textContent = "다운로드 에러";
        console.error("[다운로드] 에러:", e);
      }
    });
  }

  // Quarto 설치 상태 indicator는 항상 동작
  const indicator = document.getElementById("quarto-indicator");
  if (indicator && typeof invoke === "function") {
    invoke("check_quarto_installed")
      .then((ver) => {
        indicator.style.color = "#4caf50";
      })
      .catch((err) => {
        indicator.style.color = "#f44336";
      });
  }
});
