(function () {
  "use strict";

  const invoke = async (cmd, args = {}) => {
    if (window.__TAURI__?.core?.invoke) {
      return window.__TAURI__.core.invoke(cmd, args);
    }
    if (window.__TAURI_INTERNALS__?.invoke) {
      return window.__TAURI_INTERNALS__.invoke(cmd, args);
    }
    throw new Error("Tauri bridge not available");
  };

  const planBadge = document.getElementById("plan-badge");
  const authMessage = document.getElementById("auth-message");
  const prereqMessage = document.getElementById("prereq-message");
  const wireguardValue = document.getElementById("wireguard-value");
  const webview2Value = document.getElementById("webview2-value");
  const installWireguardBtn = document.getElementById("install-wireguard-btn");
  const installWebview2Btn = document.getElementById("install-webview2-btn");
  const refreshPrereqsBtn = document.getElementById("refresh-prereqs-btn");
  const loggedInValue = document.getElementById("logged-in-value");
  const vpnStateValue = document.getElementById("vpn-state-value");
  const selectedRouteValue = document.getElementById("selected-route-value");
  const selectedRouteSummary = document.getElementById("selected-route-summary");
  const routeTitle = document.getElementById("route-title");
  const routeSubtitle = document.getElementById("route-subtitle");
  const residentialList = document.getElementById("residential-list");
  const standardList = document.getElementById("standard-list");
  const authSurface = document.querySelector(".auth-surface");
  const serverSurface = document.querySelector(".server-surface");
  const scannerVideo = document.getElementById("scanner-video");
  const scannerStatus = document.getElementById("scanner-status");
  const startScanBtn = document.getElementById("start-scan-btn");
  const stopScanBtn = document.getElementById("stop-scan-btn");
  const pairLinkInput = document.getElementById("pair-link-input");
  const codeInput = document.getElementById("code-input");
  const emailInput = document.getElementById("email-input");
  const passwordInput = document.getElementById("password-input");
  const connectBtn = document.getElementById("connect-btn");
  const disconnectBtn = document.getElementById("disconnect-btn");
  const refreshServersBtn = document.getElementById("refresh-servers-btn");
  const createCodeBtn = document.getElementById("create-code-btn");
  const newCodeOutput = document.getElementById("new-code-output");

  let selectedServer = null;
  let cachedServers = [];
  let scanStream = null;
  let scanFrame = null;
  let scannerRunning = false;
  let prereqs = {
    wireguard_installed: false,
    webview2_installed: false
  };

  function setMessage(text, isError = false) {
    authMessage.textContent = text || "";
    authMessage.style.color = isError ? "#b24a36" : "#666157";
  }

  function setPrereqMessage(text, isError = false) {
    prereqMessage.textContent = text || "";
    prereqMessage.style.color = isError ? "#b24a36" : "#666157";
  }

  function formatTier(tier) {
    switch ((tier || "").toLowerCase()) {
      case "free":
        return "Free";
      case "escudo":
        return "Escudo";
      case "pro":
        return "Power / Family";
      case "dedicated":
        return "Dedicated";
      default:
        return "Free";
    }
  }

  function formatVpnLabel(status) {
    if (status?.vpn?.connected) return "Protected";
    return status?.logged_in ? "Not connected" : "Offline";
  }

  function normalizeCode(value) {
    return value.replace(/\D/g, "").match(/.{1,4}/g)?.join("-")?.slice(0, 19) || "";
  }

  function categorizeServer(server) {
    const country = (server.country_code || "").toUpperCase();
    const serviceClass = (server.service_class || "").toLowerCase();
    const residential = serviceClass === "medium";
    if (residential && country === "US") return "Residential US";
    if (residential && country === "GB") return "Residential UK";
    if (residential && (country === "DE" || country === "NL")) return "Residential EU";
    return "Standard";
  }

  function renderServers() {
    residentialList.innerHTML = "";
    standardList.innerHTML = "";

    const groups = {
      residential: cachedServers.filter((server) => categorizeServer(server) !== "Standard"),
      standard: cachedServers.filter((server) => categorizeServer(server) === "Standard")
    };

    const renderCard = (server) => {
      const category = categorizeServer(server);
      const el = document.createElement("button");
      el.className = "server-card" + (selectedServer?.id === server.id ? " active" : "");
      el.innerHTML = `
        <div class="server-head">
          <div>
            <strong>${category === "Standard" ? server.location : category}</strong>
            <small>${server.location}</small>
          </div>
          <span class="server-tag">${category}</span>
        </div>
        <div class="server-meta">
          <span>${server.service_class || "Standard"}</span>
          <span>${server.load_percent}% load</span>
        </div>
      `;
      el.addEventListener("click", () => {
        selectedServer = server;
        routeTitle.textContent = category === "Standard" ? server.location : category;
        routeSubtitle.textContent = `Selected ${category} route on ${server.location}.`;
        selectedRouteValue.textContent = category === "Standard" ? server.location : category;
        selectedRouteSummary.textContent = `${category === "Standard" ? server.location : category} selected`;
        connectBtn.disabled = !prereqs.wireguard_installed;
        renderServers();
      });
      return el;
    };

    groups.residential.forEach((server) => residentialList.appendChild(renderCard(server)));
    groups.standard.forEach((server) => standardList.appendChild(renderCard(server)));

    if (!groups.residential.length) {
      residentialList.innerHTML = `<div class="server-card"><strong>No residential routes</strong><small>Sign in and refresh after the backend responds.</small></div>`;
    }
    if (!groups.standard.length) {
      standardList.innerHTML = `<div class="server-card"><strong>No standard servers</strong><small>Sign in and refresh after the backend responds.</small></div>`;
    }
  }

  async function refreshStatus() {
    try {
      const status = await invoke("get_status");
      loggedInValue.textContent = status.logged_in ? "Signed in" : "Offline";
      vpnStateValue.textContent = formatVpnLabel(status);
      selectedRouteValue.textContent = status.vpn.server_name || "None";
      selectedRouteSummary.textContent = status.vpn.server_name || (selectedServer ? selectedServer.location : "No route selected");
      disconnectBtn.disabled = !status.vpn.connected;
      authSurface.style.display = status.logged_in ? "none" : "";
      serverSurface.style.display = status.logged_in ? "" : "none";

      if (status.logged_in) {
        try {
          const launch = await invoke("get_launch_status");
          planBadge.textContent = formatTier(launch.effective_tier);
        } catch (_) {
          planBadge.textContent = "Signed in";
        }
      } else {
        planBadge.textContent = "Not signed in";
      }
    } catch (error) {
      setMessage(error.message || String(error), true);
    }
  }

  async function refreshPrereqs() {
    try {
      prereqs = await invoke("check_windows_prereqs");
      wireguardValue.textContent = prereqs.wireguard_installed ? "Ready" : "Missing";
      webview2Value.textContent = prereqs.webview2_installed ? "Ready" : "Missing";
      wireguardValue.className = prereqs.wireguard_installed ? "ready" : "missing";
      webview2Value.className = prereqs.webview2_installed ? "ready" : "missing";
      connectBtn.disabled = !selectedServer || !prereqs.wireguard_installed;
      setPrereqMessage(
        prereqs.wireguard_installed && prereqs.webview2_installed
          ? "Windows is ready."
          : "Install the missing components once, then pair and connect."
      );
      const prereqSurface = document.querySelector(".prereq-surface");
      if (prereqSurface) {
        prereqSurface.style.display = prereqs.wireguard_installed && prereqs.webview2_installed ? "none" : "";
      }
    } catch (error) {
      setPrereqMessage(error.message || String(error), true);
    }
  }

  async function refreshServers() {
    try {
      cachedServers = await invoke("get_servers");
      renderServers();
    } catch (error) {
      setMessage(error.message || String(error), true);
    }
  }

  async function handleAuth(action) {
    try {
      setMessage("");
      if (action === "code") {
        await invoke("login_number", { accountNumber: normalizeCode(codeInput.value) });
      } else if (action === "email") {
        await invoke("login", { email: emailInput.value.trim(), password: passwordInput.value });
      } else if (action === "register") {
        await invoke("register", { email: emailInput.value.trim(), password: passwordInput.value });
      } else if (action === "pair-link") {
        await invoke("scan_qr", { rawValue: pairLinkInput.value.trim() });
      }
      setMessage("Signed in on Windows.");
      await refreshStatus();
      await refreshServers();
      routeSubtitle.textContent = "Choose a route below, then press the power button.";
    } catch (error) {
      setMessage(error.message || String(error), true);
    }
  }

  async function createFreeAccount() {
    try {
      const code = await invoke("create_anonymous_account");
      newCodeOutput.textContent = `New free account: ${code}`;
      newCodeOutput.classList.remove("hidden");
      codeInput.value = normalizeCode(code);
      setMessage("Free account created. You can sign in with that code now.");
    } catch (error) {
      setMessage(error.message || String(error), true);
    }
  }

  async function connectSelected() {
    if (!selectedServer) {
      setMessage("Choose a route first.", true);
      return;
    }
    if (!prereqs.wireguard_installed) {
      setMessage("Install the secure network component first.", true);
      return;
    }
    try {
      await invoke("connect", {
        serverId: selectedServer.id,
        serverName: selectedServer.name,
        serverLocation: selectedServer.location
      });
      setMessage(`Connected to ${selectedServer.location}.`);
      selectedRouteSummary.textContent = `${selectedServer.location} active`;
      routeSubtitle.textContent = "Your traffic is encrypted and your selected exit is live.";
      await refreshStatus();
    } catch (error) {
      setMessage(error.message || String(error), true);
    }
  }

  async function disconnectCurrent() {
    try {
      await invoke("disconnect");
      setMessage("Disconnected.");
      selectedRouteSummary.textContent = selectedServer ? `${selectedServer.location} selected` : "No route selected";
      routeSubtitle.textContent = "Choose a route below, then press the power button.";
      await refreshStatus();
    } catch (error) {
      setMessage(error.message || String(error), true);
    }
  }

  async function stopScanner() {
    scannerRunning = false;
    if (scanFrame) {
      cancelAnimationFrame(scanFrame);
      scanFrame = null;
    }
    if (scanStream) {
      scanStream.getTracks().forEach((track) => track.stop());
      scanStream = null;
    }
    stopScanBtn.disabled = true;
    startScanBtn.disabled = false;
    scannerStatus.textContent = "Scanner stopped.";
  }

  async function startScanner() {
    try {
      if (!prereqs.webview2_installed) {
        setMessage("Install the app runtime first.", true);
        return;
      }
      if (!("BarcodeDetector" in window)) {
        setMessage("This Windows WebView does not support live QR detection. Paste the pairing link instead.", true);
        return;
      }
      const detector = new BarcodeDetector({ formats: ["qr_code"] });
      scanStream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment" },
        audio: false
      });
      scannerVideo.srcObject = scanStream;
      scannerRunning = true;
      startScanBtn.disabled = true;
      stopScanBtn.disabled = false;
      scannerStatus.textContent = "Camera live. Point it at the QR on your Android phone.";

      const loop = async () => {
        if (!scannerRunning) return;
        try {
          const codes = await detector.detect(scannerVideo);
          if (codes.length && codes[0].rawValue) {
            const raw = codes[0].rawValue;
            scannerStatus.textContent = "QR detected. Pairing now...";
            await stopScanner();
            pairLinkInput.value = raw;
            await handleAuth("pair-link");
            return;
          }
        } catch (_) {}
        scanFrame = requestAnimationFrame(loop);
      };

      scanFrame = requestAnimationFrame(loop);
    } catch (error) {
      setMessage(error.message || String(error), true);
      scannerStatus.textContent = "Could not open the camera.";
      await stopScanner();
    }
  }

  document.querySelectorAll(".mode-chip").forEach((button) => {
    button.addEventListener("click", () => {
      document.querySelectorAll(".mode-chip").forEach((chip) => chip.classList.remove("active"));
      document.querySelectorAll(".mode-panel").forEach((panel) => panel.classList.remove("active"));
      button.classList.add("active");
      document.getElementById(`mode-${button.dataset.mode}`).classList.add("active");
    });
  });

  codeInput.addEventListener("input", () => {
    codeInput.value = normalizeCode(codeInput.value);
  });

  document.getElementById("code-login-btn").addEventListener("click", () => handleAuth("code"));
  document.getElementById("email-login-btn").addEventListener("click", () => handleAuth("email"));
  document.getElementById("register-btn").addEventListener("click", () => handleAuth("register"));
  document.getElementById("pair-link-btn").addEventListener("click", () => handleAuth("pair-link"));
  createCodeBtn.addEventListener("click", createFreeAccount);
  connectBtn.addEventListener("click", connectSelected);
  disconnectBtn.addEventListener("click", disconnectCurrent);
  refreshServersBtn.addEventListener("click", refreshServers);
  startScanBtn.addEventListener("click", startScanner);
  stopScanBtn.addEventListener("click", stopScanner);
  installWireguardBtn.addEventListener("click", async () => {
    try {
      setPrereqMessage("Starting secure network component install...");
      await invoke("install_wireguard");
      setPrereqMessage("WireGuard installer launched. Finish it, then click Refresh.");
    } catch (error) {
      setPrereqMessage(error.message || String(error), true);
    }
  });
  installWebview2Btn.addEventListener("click", async () => {
    try {
      setPrereqMessage("Starting app runtime install...");
      await invoke("install_webview2");
      setPrereqMessage("WebView2 installer launched. Finish it, then click Refresh.");
    } catch (error) {
      setPrereqMessage(error.message || String(error), true);
    }
  });
  refreshPrereqsBtn.addEventListener("click", refreshPrereqs);

  window.addEventListener("beforeunload", () => {
    stopScanner();
  });

  refreshPrereqs();
  refreshStatus();
})();
