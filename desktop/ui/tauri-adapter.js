(function installTauriLifecycleAdapter(global) {
  "use strict";

  const bridge = global.OSWispaDesktopBridge;
  const tauriEvents = global.__TAURI__?.event;
  if (!bridge || typeof tauriEvents?.listen !== "function") return;

  const preview = bridge.createDevelopmentAdapter();
  const nativeUnlisteners = new Set();
  let disposed = false;

  function invoke(command, payload = {}) {
    // Settings, Ready Check and History remain explicitly preview-only until
    // their native persistence and privacy boundaries are implemented.
    return preview.invoke(command, payload);
  }

  function listen(eventName, handler) {
    if (eventName !== bridge.EVENTS.LIFECYCLE) {
      return preview.listen(eventName, handler);
    }
    if (typeof handler !== "function") throw new TypeError("Bridge listener must be a function");

    let unlisten = null;
    let listenerDisposed = false;
    tauriEvents.listen(eventName, (event) => {
      if (disposed || listenerDisposed) return;
      const state = event?.payload?.state;
      if (typeof state === "string") handler(Object.freeze({ state }));
    }).then((nativeUnlisten) => {
      if (disposed || listenerDisposed) nativeUnlisten();
      else {
        unlisten = nativeUnlisten;
        nativeUnlisteners.add(nativeUnlisten);
      }
    }).catch(() => {
      // The UI remains usable as a preview if the narrow lifecycle capability
      // is unavailable. No native error detail is exposed to the webview.
    });

    return function unlistenLifecycle() {
      listenerDisposed = true;
      if (unlisten) {
        nativeUnlisteners.delete(unlisten);
        unlisten();
      }
    };
  }

  function dispose() {
    disposed = true;
    preview.dispose();
    for (const unlisten of nativeUnlisteners) unlisten();
    nativeUnlisteners.clear();
  }

  bridge.installAdapter(Object.freeze({ invoke, listen, dispose, mode: "native-lifecycle" }));
  document.documentElement.dataset.bridge = "native";

  document.querySelector(".development-banner")?.replaceChildren(
    document.createTextNode("Desktop shell foundation · lifecycle is live · settings, Ready Check and History remain preview-only"),
  );
  const signalBoundary = document.querySelector(".signal-preview-label");
  if (signalBoundary) signalBoundary.textContent = "Lifecycle only · transcript hidden";
})(window);
