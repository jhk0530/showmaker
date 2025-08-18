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
      // 확장자 체크: .md만 허용
      if (!file.name.toLowerCase().endsWith(".md")) {
        showNotification("Only .md files are allowed!");
        e.target.value = "";
        if (mdHeaderInfo) mdHeaderInfo.innerHTML = "";
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

        try {
          const [fileName, base64] = await invoke("download_rendered_html", {
            htmlPath: lastHtmlPath,
          });
          const saveName = fileName;
          try {
            const result = await invoke("save_html_file", {
              htmlPath: lastHtmlPath,
              defaultName: saveName,
            });
            console.log(result);
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
        } catch (e) {
          handleError("[Download] Error:", e);
        }
      } catch (e) {
        handleError("Quarto render failed:", e);
      }
    });
  }

  const indicator = document.getElementById("quarto-indicator");
  function setQuartoUI(installed) {
    if (!indicator) return;
    if (installed) {
      indicator.style.color = "#4caf50";
      if (uploadLabel) {
        uploadLabel.classList.remove("disabled");
        uploadLabel.title = "Upload";
        uploadLabel.style.pointerEvents = "auto";
        uploadLabel.style.opacity = "1";
        const tooltip = document.getElementById("md-upload-tooltip");
        if (tooltip) tooltip.style.display = "none";
      }
    } else {
      indicator.style.color = "#f44336";
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
  if (indicator && typeof invoke === "function") {
    invoke("check_quarto_installed")
      .then(() => setQuartoUI(true))
      .catch(() => setQuartoUI(false));
  }
});

/*
// File upload input element
const upload = document.getElementById("md-upload");
if (upload) {
  upload.addEventListener("change", (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = function (ev) {
      const mdText = ev.target.result;
      // Save to window.uploadedMarkdown here if needed
    };
    reader.readAsText(file);
  });
}
*/
// File download function
function downloadFile(filePath, fileName) {
  const a = document.createElement("a");
  a.href = filePath;
  a.download = fileName;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
}
