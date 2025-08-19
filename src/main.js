// 다운로드 및 저장 처리 함수 (다운로드 실패 시 fallback까지 포함)
async function downloadAndSave(htmlPath) {
  try {
    const [fileName, base64] = await invoke("download_rendered_html", {
      htmlPath,
    });
    const saveName = fileName;
    try {
      await invoke("save_html_file", { htmlPath, defaultName: saveName });
    } catch (saveError) {
      handleError("Tauri save failed:", saveError);
      // Fallback: browser download
      const link = document.createElement("a");
      link.href = "data:text/html;base64," + base64;
      link.download = saveName;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      console.log("HTML download complete (fallback)");
    }
  } catch (error) {
    handleError("Download or save failed:", error);
  }
}
// Dedicated error handler for logging and notification
function handleError(...args) {
  console.error(...args);
  if (args.length > 0 && typeof showNotification === "function") {
    const msg = args
      .map((arg) =>
        typeof arg === "object" ? JSON.stringify(arg) : String(arg)
      )
      .join(" ");
    showNotification(msg);
  }
}
// Simple notification function
function showNotification(message) {
  const n = document.createElement("div");
  n.className = "notification";
  n.textContent = message;
  document.body.appendChild(n);
  // Force a DOM reflow so that the browser applies the initial styles of the notification
  // before the "show" class is added. This ensures the CSS transition animates properly.
  void n.offsetWidth;
  n.classList.add("show");
  setTimeout(() => {
    n.classList.remove("show");
  }, 1800);
  setTimeout(() => {
    n.remove();
  }, 2200);
}

const { invoke } = window.__TAURI__.core;

window.addEventListener("DOMContentLoaded", () => {
  const upload = document.getElementById("md-upload");
  const uploadLabel = document.getElementById("md-upload-label");
  const renderBtn = document.getElementById("quarto-render-btn");

  let lastMdContent = null;
  let lastMdName = null;
  let lastHtmlPath = null;
  if (upload) {
    upload.addEventListener("change", (e) => {
      const file = e.target.files[0];
      if (!file) return;
      // Check file extension: only allow .md files
      if (!file.name.toLowerCase().endsWith(".md")) {
        showNotification("Only .md files are allowed!");
        e.target.value = "";

        if (renderBtn) renderBtn.style.display = "none";
        return;
      }
      const reader = new FileReader();
      reader.onload = function (ev) {
        const mdText = ev.target.result;
        lastMdContent = mdText;
        lastMdName = file.name;
      };
      reader.readAsText(file);
      if (renderBtn) renderBtn.style.display = "inline-flex";
    });
  }
  if (renderBtn) {
    renderBtn.addEventListener("click", async () => {
      if (renderBtn.disabled) return;
      if (!lastMdContent) {
        console.log("Please upload a markdown file first.");
        return;
      }
      try {
        const htmlPath = await invoke("render_quarto_file", {
          mdContent: lastMdContent,
          origName: lastMdName,
        });
        lastHtmlPath = htmlPath;
        await downloadAndSave(htmlPath);
      } catch (e) {
        handleError("Quarto render failed:", e);
      }
    });
  }

  function setQuartoUI(installed) {
    if (installed) {
      if (uploadLabel) {
        uploadLabel.classList.remove("disabled");
        uploadLabel.title = "Upload";
        uploadLabel.style.pointerEvents = "auto";
        uploadLabel.style.opacity = "1";
        const tooltip = document.getElementById("md-upload-tooltip");
        if (tooltip) tooltip.style.display = "none";
      }
    } else {
      if (uploadLabel) {
        uploadLabel.classList.add("disabled");
        uploadLabel.title = "";
        uploadLabel.style.pointerEvents = "none";
        uploadLabel.style.opacity = "0.5";
        const tooltip = document.getElementById("md-upload-tooltip");
        if (tooltip) tooltip.style.display = "block";
      }
    }
  }
  if (typeof invoke === "function") {
    invoke("check_quarto_installed")
      .then(() => setQuartoUI(true))
      .catch(() => setQuartoUI(false));
  }
});
