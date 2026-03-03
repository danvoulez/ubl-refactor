const PANEL_MAP = {
  chat: {
    title: "AI Chat com Agente Chefe",
    subtitle: "Acompanhamento estratégico e decisões assistidas",
    panelId: "chat-panel",
  },
  observability: {
    title: "Observabilidade Total",
    subtitle: "SLO, incidentes, métricas e sinais de confiança em tempo real",
    panelId: "observability-panel",
  },
  settings: {
    title: "Configurações Premium",
    subtitle: "Controle fino de AI, telemetria e segurança",
    panelId: "settings-panel",
  },
};

const incidentData = [
  { title: "Pico de latência no cluster tr-west", time: "há 12 min", status: "Mitigado" },
  { title: "Tentativa de acesso sem claim obrigatória", time: "há 43 min", status: "Bloqueado" },
  { title: "Rotação automática de segredo completada", time: "há 1h", status: "Concluído" },
];

const chiefResponses = [
  "Como agente chefe, recomendo tratar confiabilidade como produto: priorize o erro 5xx e degrade com elegância.",
  "Sugestão executiva: crie um ritual diário de revisão de incidentes com foco em causa raiz e ação preventiva.",
  "Você está operando bem. Próximo salto premium: correlacionar métricas técnicas com impacto no negócio em um painel único.",
];

const nodes = {
  menuItems: Array.from(document.querySelectorAll(".menu-item")),
  panels: Array.from(document.querySelectorAll(".panel")),
  panelTitle: document.getElementById("panel-title"),
  panelSubtitle: document.getElementById("panel-subtitle"),
  chatMessages: document.getElementById("chat-messages"),
  chatForm: document.getElementById("chat-form"),
  chatInput: document.getElementById("chat-input"),
  incidentList: document.getElementById("incident-list"),
  settingsForm: document.getElementById("settings-form"),
  saveStatus: document.getElementById("save-status"),
  temperature: document.getElementById("temperature"),
  temperatureValue: document.getElementById("temperature-value"),
};

const defaultSettings = {
  temperature: "0.3",
  longReplies: true,
  sessionMemory: true,
  refreshSeconds: 15,
  liveAlerts: true,
  telemetryMode: "strict",
  accessProfile: "Chief Operator",
  mfaRequired: true,
};

function setActivePanel(panelKey) {
  const panelCfg = PANEL_MAP[panelKey];
  if (!panelCfg) return;

  nodes.menuItems.forEach((item) => {
    item.classList.toggle("active", item.dataset.panel === panelKey);
  });

  nodes.panels.forEach((panel) => {
    panel.classList.toggle("active", panel.id === panelCfg.panelId);
  });

  nodes.panelTitle.textContent = panelCfg.title;
  nodes.panelSubtitle.textContent = panelCfg.subtitle;
}

function addMessage(role, text) {
  const el = document.createElement("div");
  el.className = `message ${role}`;
  el.textContent = text;
  nodes.chatMessages.appendChild(el);
  nodes.chatMessages.scrollTop = nodes.chatMessages.scrollHeight;
}

function loadIncidents() {
  nodes.incidentList.innerHTML = "";
  incidentData.forEach((incident) => {
    const el = document.createElement("article");
    el.className = "incident";
    el.innerHTML = `<strong>${incident.title}</strong><p>${incident.time} · ${incident.status}</p>`;
    nodes.incidentList.appendChild(el);
  });
}

function readSettings() {
  try {
    return {
      ...defaultSettings,
      ...JSON.parse(localStorage.getItem("ubl-premium-settings") || "{}"),
    };
  } catch {
    return defaultSettings;
  }
}

function writeSettings(settings) {
  localStorage.setItem("ubl-premium-settings", JSON.stringify(settings));
}

function hydrateSettingsForm() {
  const settings = readSettings();
  document.getElementById("temperature").value = settings.temperature;
  document.getElementById("long-replies").checked = settings.longReplies;
  document.getElementById("session-memory").checked = settings.sessionMemory;
  document.getElementById("refresh-seconds").value = settings.refreshSeconds;
  document.getElementById("live-alerts").checked = settings.liveAlerts;
  document.getElementById("telemetry-mode").value = settings.telemetryMode;
  document.getElementById("access-profile").value = settings.accessProfile;
  document.getElementById("mfa-required").checked = settings.mfaRequired;
  nodes.temperatureValue.textContent = settings.temperature;
}

function wireEvents() {
  nodes.menuItems.forEach((item) => {
    item.addEventListener("click", () => setActivePanel(item.dataset.panel));
  });

  nodes.chatForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const content = nodes.chatInput.value.trim();
    if (!content) return;
    addMessage("user", content);
    nodes.chatInput.value = "";

    const response = chiefResponses[Math.floor(Math.random() * chiefResponses.length)];
    setTimeout(() => addMessage("bot", response), 300);
  });

  nodes.settingsForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const settings = {
      temperature: document.getElementById("temperature").value,
      longReplies: document.getElementById("long-replies").checked,
      sessionMemory: document.getElementById("session-memory").checked,
      refreshSeconds: Number(document.getElementById("refresh-seconds").value),
      liveAlerts: document.getElementById("live-alerts").checked,
      telemetryMode: document.getElementById("telemetry-mode").value,
      accessProfile: document.getElementById("access-profile").value,
      mfaRequired: document.getElementById("mfa-required").checked,
    };

    writeSettings(settings);
    nodes.saveStatus.textContent = "Configurações salvas com sucesso.";
    setTimeout(() => {
      nodes.saveStatus.textContent = "";
    }, 2500);
  });

  nodes.temperature.addEventListener("input", () => {
    nodes.temperatureValue.textContent = nodes.temperature.value;
  });
}

function boot() {
  loadIncidents();
  hydrateSettingsForm();
  wireEvents();
  addMessage("bot", "Olá. Eu sou seu agente chefe. Posso ajudar com estratégia, incidentes e governança.");
  addMessage("bot", "Status atual: plataforma estável, sem alertas críticos ativos.");
}

boot();
