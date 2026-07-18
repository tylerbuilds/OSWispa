(function installBridgeContract(global) {
  "use strict";

  const COMMANDS = Object.freeze({
    READ_BOOTSTRAP: "read_bootstrap",
    SAVE_SETTINGS: "save_settings",
    RUN_READY_CHECK: "run_ready_check",
    COPY_HISTORY_ENTRY: "copy_history_entry",
    CLEAR_HISTORY: "clear_history",
  });

  const EVENTS = Object.freeze({
    LIFECYCLE: "lifecycle",
    READY_CHECK_CHANGED: "ready_check_changed",
    SETTINGS_CHANGED: "settings_changed",
    HISTORY_CHANGED: "history_changed",
  });

  const REQUIRED_ADAPTER_METHODS = Object.freeze(["invoke", "listen", "dispose"]);
  let activeAdapter = null;

  function clonePreviewValue(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function assertAdapter(adapter) {
    if (!adapter || typeof adapter !== "object") {
      throw new TypeError("MorpheOS Voice UI adapter must be an object");
    }
    for (const method of REQUIRED_ADAPTER_METHODS) {
      if (typeof adapter[method] !== "function") {
        throw new TypeError(`MorpheOS Voice UI adapter is missing ${method}()`);
      }
    }
  }

  function installAdapter(adapter) {
    assertAdapter(adapter);
    if (activeAdapter) activeAdapter.dispose();
    activeAdapter = adapter;
    return activeAdapter;
  }

  function getAdapter() {
    if (!activeAdapter) {
      throw new Error("MorpheOS Voice UI bridge has no installed adapter");
    }
    return activeAdapter;
  }

  function invoke(command, payload = {}) {
    return getAdapter().invoke(command, payload);
  }

  function listen(eventName, handler) {
    return getAdapter().listen(eventName, handler);
  }

  function createDevelopmentAdapter() {
    const listeners = new Map();
    const previewState = {
      settings: {
        auto_insert: true,
        signal_enabled: true,
        feedback_tones: true,
        history_enabled: true,
        transcript_preview: false,
        remote_enabled: false,
      },
      ready: {
        privacy_model: "ready",
        microphone: "not_checked",
        shortcut_insertion: "ready",
        sample_receipt: "not_checked",
      },
      history_count: 3,
    };

    function emit(eventName, detail) {
      const eventListeners = listeners.get(eventName) || new Set();
      for (const handler of eventListeners) handler(detail);
    }

    async function invokeDevelopment(command, payload = {}) {
      switch (command) {
        case COMMANDS.READ_BOOTSTRAP:
          return clonePreviewValue(previewState);
        case COMMANDS.SAVE_SETTINGS:
          previewState.settings = { ...previewState.settings, ...payload };
          emit(EVENTS.SETTINGS_CHANGED, clonePreviewValue(previewState.settings));
          return { persisted: false, mode: "development" };
        case COMMANDS.RUN_READY_CHECK: {
          const checkId = String(payload.check_id || "");
          if (!Object.prototype.hasOwnProperty.call(previewState.ready, checkId)) {
            throw new Error(`Unknown development ready check: ${checkId}`);
          }
          previewState.ready[checkId] = "ready";
          const receipt = checkId === "sample_receipt"
            ? { id: "sample-0042", transcript_visible: false, delivery: "inserted" }
            : null;
          const result = { check_id: checkId, status: "ready", receipt, simulated: true };
          emit(EVENTS.READY_CHECK_CHANGED, result);
          return result;
        }
        case COMMANDS.COPY_HISTORY_ENTRY:
          return { copied: false, mode: "development", receipt_id: payload.receipt_id || "" };
        case COMMANDS.CLEAR_HISTORY:
          previewState.history_count = 0;
          emit(EVENTS.HISTORY_CHANGED, { count: 0, simulated: true });
          return { cleared: true, persisted: false, mode: "development" };
        default:
          throw new Error(`Unsupported MorpheOS Voice UI command: ${command}`);
      }
    }

    function listenDevelopment(eventName, handler) {
      if (typeof handler !== "function") throw new TypeError("Bridge listener must be a function");
      const eventListeners = listeners.get(eventName) || new Set();
      eventListeners.add(handler);
      listeners.set(eventName, eventListeners);
      return function unlisten() {
        eventListeners.delete(handler);
      };
    }

    function disposeDevelopment() {
      listeners.clear();
    }

    return Object.freeze({
      invoke: invokeDevelopment,
      listen: listenDevelopment,
      dispose: disposeDevelopment,
      mode: "development",
    });
  }

  global.MorpheOSVoiceDesktopBridge = Object.freeze({
    COMMANDS,
    EVENTS,
    REQUIRED_ADAPTER_METHODS,
    installAdapter,
    getAdapter,
    invoke,
    listen,
    createDevelopmentAdapter,
  });
})(window);
