(function runDesktopFoundation(global) {
  "use strict";

  const bridge = global.MorpheOSVoiceDesktopBridge;
  if (!bridge) return;

  if (document.documentElement.dataset.bridge === "development") {
    bridge.installAdapter(bridge.createDevelopmentAdapter());
  }

  const SIGNAL_STATES = Object.freeze({
    booting: Object.freeze({ label: "Starting", detail: "Checking local setup", receipt: "Local" }),
    ready: Object.freeze({ label: "Ready", detail: "Hold Ctrl + Super to speak", receipt: "Local" }),
    arming: Object.freeze({ label: "Arming", detail: "Preparing the microphone", receipt: "Input" }),
    listening: Object.freeze({ label: "Listening", detail: "Release the shortcut to transcribe", receipt: "Local" }),
    processing: Object.freeze({ label: "Processing", detail: "Transcribing on this computer", receipt: "Local" }),
    delivering: Object.freeze({ label: "Delivering", detail: "Verifying text reached the focused app", receipt: "Check" }),
    inserted: Object.freeze({ label: "Inserted", detail: "Text reached the focused app", receipt: "Verified" }),
    copied: Object.freeze({ label: "Copied", detail: "Paste manually when ready", receipt: "Fallback" }),
    cancelled: Object.freeze({ label: "Cancelled", detail: "No text was delivered", receipt: "Local" }),
    needs_attention: Object.freeze({ label: "Needs attention", detail: "Open MorpheOS Voice for the recovery step", receipt: "Check" }),
  });

  function setView(viewId, options = {}) {
    const target = document.querySelector(`[data-view="${viewId}"]`);
    if (!target) return;

    document.querySelectorAll("[data-view]").forEach((view) => {
      view.hidden = view !== target;
    });
    document.querySelectorAll("[data-view-button]").forEach((button) => {
      const active = button.dataset.viewButton === viewId;
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-pressed", String(active));
    });

    if (options.updateHash !== false) history.replaceState(null, "", `#${viewId}`);
    if (options.focus) target.querySelector("h2")?.focus({ preventScroll: true });
  }

  function initialiseViewNavigation() {
    const views = document.querySelectorAll("[data-view]");
    if (!views.length) return;

    document.querySelectorAll("[data-view-button]").forEach((button) => {
      button.addEventListener("click", () => setView(button.dataset.viewButton));
    });
    document.querySelectorAll("[data-view-jump]").forEach((button) => {
      button.addEventListener("click", () => setView(button.dataset.viewJump));
    });

    const requested = global.location.hash.slice(1);
    if (requested && document.querySelector(`[data-view="${requested}"]`)) {
      setView(requested, { updateHash: false });
    }
  }

  function activateSettingsTab(tabId, moveFocus = false) {
    const tabs = [...document.querySelectorAll("[data-settings-tab]")];
    const activeTab = tabs.find((tab) => tab.dataset.settingsTab === tabId);
    if (!activeTab) return;

    for (const tab of tabs) {
      const selected = tab === activeTab;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
    }
    document.querySelectorAll("[data-settings-panel]").forEach((panel) => {
      panel.hidden = panel.dataset.settingsPanel !== tabId;
    });
    if (moveFocus) activeTab.focus();
  }

  function initialiseSettingsTabs() {
    const tabs = [...document.querySelectorAll("[data-settings-tab]")];
    if (!tabs.length) return;

    tabs.forEach((tab, index) => {
      tab.addEventListener("click", () => activateSettingsTab(tab.dataset.settingsTab));
      tab.addEventListener("keydown", (event) => {
        const keys = ["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft", "Home", "End"];
        if (!keys.includes(event.key)) return;
        event.preventDefault();

        let nextIndex = index;
        if (event.key === "ArrowDown" || event.key === "ArrowRight") nextIndex = (index + 1) % tabs.length;
        if (event.key === "ArrowUp" || event.key === "ArrowLeft") nextIndex = (index - 1 + tabs.length) % tabs.length;
        if (event.key === "Home") nextIndex = 0;
        if (event.key === "End") nextIndex = tabs.length - 1;
        activateSettingsTab(tabs[nextIndex].dataset.settingsTab, true);
      });
    });
  }

  function updateReadyCount() {
    const steps = [...document.querySelectorAll("[data-ready-step]")];
    const ready = steps.filter((step) => step.classList.contains("is-ready")).length;
    const count = document.querySelector("[data-ready-count]");
    if (count) count.textContent = `${ready} / ${steps.length}`;
    const score = document.querySelector("[data-readiness-score]");
    if (score) score.setAttribute("aria-label", `Preview readiness: ${ready} of ${steps.length} checks ready`);
  }

  async function runPreviewCheck(checkId, trigger) {
    trigger.disabled = true;
    const originalLabel = trigger.textContent;
    trigger.textContent = "Running preview…";
    try {
      const result = await bridge.invoke(bridge.COMMANDS.RUN_READY_CHECK, { check_id: checkId });
      const step = document.querySelector(`[data-ready-step="${result.check_id}"]`);
      if (step) {
        step.classList.add("is-ready");
        const status = step.querySelector("[data-step-status]");
        if (status) {
          status.textContent = "Preview ready";
          status.classList.add("status-pill--ready");
        }
      }
      if (checkId === "microphone") {
        document.querySelectorAll("[data-ready-detail]").forEach((detail) => {
          detail.textContent = "Preview USB microphone · synthetic ready receipt";
        });
      }
      if (checkId === "sample_receipt") {
        const receipt = document.querySelector("[data-sample-receipt]");
        if (receipt) receipt.hidden = false;
      }
      updateReadyCount();
      trigger.textContent = "Preview state shown";
    } catch (error) {
      trigger.textContent = "Preview unavailable";
      console.error(error);
    } finally {
      global.setTimeout(() => {
        trigger.disabled = false;
        trigger.textContent = originalLabel;
      }, 1600);
    }
  }

  function initialiseReadyChecks() {
    document.querySelectorAll("[data-preview-check]").forEach((button) => {
      button.addEventListener("click", () => runPreviewCheck(button.dataset.previewCheck, button));
    });
    updateReadyCount();
  }

  function serialisePreviewSettings(form) {
    const settings = {};
    for (const element of form.elements) {
      if (!element.name || element.disabled) continue;
      if (element.type === "checkbox") settings[element.name] = element.checked;
      else if (element.type === "radio") {
        if (element.checked) settings[element.name] = element.value;
      } else settings[element.name] = element.value;
    }
    return settings;
  }

  function initialiseSettingsForm() {
    const form = document.querySelector("[data-settings-form]");
    if (!form) return;
    const status = document.querySelector("[data-save-status]");

    form.addEventListener("change", () => {
      if (status) status.textContent = "Preview settings changed · not persisted";
    });
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      const submit = form.querySelector('[type="submit"]');
      submit.disabled = true;
      try {
        const result = await bridge.invoke(bridge.COMMANDS.SAVE_SETTINGS, serialisePreviewSettings(form));
        if (status) status.textContent = result.persisted
          ? "Settings saved"
          : "Preview settings saved in memory only";
      } catch (error) {
        if (status) status.textContent = "Preview settings could not be saved";
        console.error(error);
      } finally {
        submit.disabled = false;
      }
    });
  }

  function initialiseDictionaryPreview() {
    const list = document.querySelector("[data-dictionary-list]");
    const addButton = document.querySelector("[data-add-term]");
    if (!list || !addButton) return;

    list.addEventListener("click", (event) => {
      const remove = event.target.closest("[data-remove-term]");
      if (!remove) return;
      remove.closest(".dictionary-entry")?.remove();
    });

    addButton.addEventListener("click", () => {
      const term = document.querySelector('[name="dictionary_term"]');
      const spoken = document.querySelector('[name="dictionary_spoken"]');
      const preferred = term.value.trim();
      const replacement = spoken.value.trim();
      if (!preferred || !replacement) {
        term.focus();
        return;
      }

      const row = document.createElement("div");
      row.className = "dictionary-entry";
      const copy = document.createElement("div");
      const strong = document.createElement("strong");
      strong.textContent = preferred;
      const small = document.createElement("small");
      small.textContent = `Replace: ${replacement}`;
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "text-button";
      remove.dataset.removeTerm = preferred;
      remove.textContent = "Remove preview term";
      copy.append(strong, small);
      row.append(copy, remove);
      list.append(row);
      term.value = "";
      spoken.value = "";
      term.focus();
    });
  }

  function updateSignal(root, stateId) {
    const state = SIGNAL_STATES[stateId] || SIGNAL_STATES.ready;
    const resolvedId = SIGNAL_STATES[stateId] ? stateId : "ready";
    root.dataset.state = resolvedId;
    root.querySelector("[data-signal-label]").textContent = state.label;
    root.querySelector("[data-signal-detail]").textContent = state.detail;
    root.querySelector("[data-signal-receipt]").textContent = state.receipt;
  }

  function initialiseSignal() {
    const root = document.querySelector("[data-signal-root]");
    if (!root) return;
    const requested = new URLSearchParams(global.location.search).get("state") || "ready";
    updateSignal(root, requested);
    bridge.listen(bridge.EVENTS.LIFECYCLE, (event) => updateSignal(root, event.state));
  }

  function applyHistoryFilter() {
    const query = (document.querySelector("[data-history-search]")?.value || "").trim().toLowerCase();
    const outcome = document.querySelector("[data-history-filter]")?.value || "all";
    const entries = [...document.querySelectorAll("[data-history-entry]")];
    let visible = 0;

    for (const entry of entries) {
      const queryMatches = !query || entry.dataset.searchText.includes(query);
      const outcomeMatches = outcome === "all" || entry.dataset.outcome === outcome;
      entry.hidden = !(queryMatches && outcomeMatches);
      if (!entry.hidden) visible += 1;
    }

    const empty = document.querySelector("[data-history-empty]");
    if (empty) empty.hidden = visible !== 0;
    const status = document.querySelector("[data-history-status]");
    if (status) status.textContent = `${visible} synthetic ${visible === 1 ? "entry" : "entries"}. Transcript text remains collapsed.`;
  }

  function initialiseHistory() {
    const list = document.querySelector("[data-history-list]");
    if (!list) return;
    document.querySelector("[data-history-search]")?.addEventListener("input", applyHistoryFilter);
    document.querySelector("[data-history-filter]")?.addEventListener("change", applyHistoryFilter);

    list.addEventListener("click", async (event) => {
      const copy = event.target.closest("[data-copy-entry]");
      if (!copy) return;
      const result = await bridge.invoke(bridge.COMMANDS.COPY_HISTORY_ENTRY, { receipt_id: copy.dataset.copyEntry });
      const status = document.querySelector("[data-history-status]");
      if (status) status.textContent = result.copied
        ? "Text copied."
        : "Development receipt acknowledged; the system clipboard was not changed.";
    });

    const dialog = document.querySelector("[data-clear-dialog]");
    function openClearDialog() {
      if (typeof dialog.showModal === "function") dialog.showModal();
      else dialog.setAttribute("open", "");
    }

    function closeClearDialog() {
      if (typeof dialog.close === "function") dialog.close();
      else dialog.removeAttribute("open");
    }

    document.querySelector("[data-clear-history]")?.addEventListener("click", openClearDialog);
    document.querySelector("[data-clear-cancel]")?.addEventListener("click", closeClearDialog);
    document.querySelector("[data-clear-confirm]")?.addEventListener("click", async () => {
      await bridge.invoke(bridge.COMMANDS.CLEAR_HISTORY);
      document.querySelectorAll("[data-history-entry]").forEach((entry) => entry.remove());
      closeClearDialog();
      applyHistoryFilter();
    });
    applyHistoryFilter();
  }

  initialiseViewNavigation();
  initialiseSettingsTabs();
  initialiseReadyChecks();
  initialiseSettingsForm();
  initialiseDictionaryPreview();
  initialiseSignal();
  initialiseHistory();
})(window);
