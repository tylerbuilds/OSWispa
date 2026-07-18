document.documentElement.classList.add("js");

window.addEventListener("load", () => {
  document.documentElement.classList.add("loaded");

  const params = new URLSearchParams(window.location.search);
  const successBanner = document.querySelector("[data-form-success]");
  if (successBanner && params.get("sent") === "1") {
    successBanner.classList.add("is-visible");
  }
});

const navToggle = document.querySelector("[data-nav-toggle]");
const nav = document.querySelector("[data-site-nav]");

if (navToggle && nav) {
  navToggle.addEventListener("click", () => {
    const isOpen = document.documentElement.classList.toggle("nav-open");
    navToggle.setAttribute("aria-expanded", String(isOpen));
  });

  nav.addEventListener("click", (event) => {
    if (event.target instanceof HTMLElement && event.target.closest("a")) {
      document.documentElement.classList.remove("nav-open");
      navToggle.setAttribute("aria-expanded", "false");
    }
  });

  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      document.documentElement.classList.remove("nav-open");
      navToggle.setAttribute("aria-expanded", "false");
    }
  });
}

async function hydrateLatestReleaseTag() {
  const nodes = document.querySelectorAll('[data-latest-release]');
  if (!nodes.length) return;

  try {
    const res = await fetch('https://api.github.com/repos/tylerbuilds/OSWispa/releases/latest', {
      headers: {
        'Accept': 'application/vnd.github+json',
      },
    });

    if (!res.ok) return;
    const json = await res.json();
    const tag = (json && json.tag_name ? String(json.tag_name) : '').trim();
    if (!tag) return;

    nodes.forEach((node) => {
      node.textContent = tag;
    });
  } catch (_) {
    // Ignore network errors; the page already renders a sensible fallback.
  }
}

hydrateLatestReleaseTag();

function detectPlatform() {
  const platform = (
    navigator.userAgentData?.platform
    || navigator.platform
    || navigator.userAgent
    || ""
  ).toLowerCase();

  if (platform.includes("win")) return "windows";
  if (platform.includes("mac")) return "macos";
  if (platform.includes("linux") || platform.includes("x11")) return "linux";
  return "other";
}

function personalisePlatformControls() {
  const platform = detectPlatform();
  const download = {
    windows: {
      label: "Download Windows alpha",
      href: "https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa-windows-x86_64.zip",
    },
    macos: {
      label: "Choose a macOS download",
      href: "#downloads",
    },
    linux: {
      label: "Choose a Linux download",
      href: "#downloads",
    },
    other: {
      label: "View all downloads",
      href: "https://github.com/tylerbuilds/OSWispa/releases/latest",
    },
  }[platform];

  document.querySelectorAll("[data-platform-download]").forEach((node) => {
    node.textContent = download.label;
    node.setAttribute("href", download.href);
  });

  const hotkey = {
    windows: "Ctrl + Windows",
    macos: "Ctrl + Cmd",
    linux: "Ctrl + Super",
    other: "Ctrl + Super",
  }[platform];

  document.querySelectorAll("[data-platform-hotkey]").forEach((node) => {
    node.textContent = hotkey;
  });
}

personalisePlatformControls();
