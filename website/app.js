document.documentElement.classList.add("js");

const navToggle = document.querySelector("[data-nav-toggle]");
const nav = document.querySelector("[data-site-nav]");

function closeNavigation() {
  document.documentElement.classList.remove("nav-open");
  navToggle?.setAttribute("aria-expanded", "false");
}

if (navToggle && nav) {
  navToggle.addEventListener("click", () => {
    const isOpen = document.documentElement.classList.toggle("nav-open");
    navToggle.setAttribute("aria-expanded", String(isOpen));
  });

  nav.addEventListener("click", (event) => {
    if (event.target instanceof HTMLElement && event.target.closest("a")) {
      closeNavigation();
    }
  });

  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") closeNavigation();
  });
}

function detectPlatform() {
  const value = (
    navigator.userAgentData?.platform
    || navigator.platform
    || navigator.userAgent
    || ""
  ).toLowerCase();

  if (value.includes("win")) return "windows";
  if (value.includes("mac")) return "macos";
  if (value.includes("linux") || value.includes("x11")) return "linux";
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
      label: "Download MorpheOS Voice",
      href: "#downloads",
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

async function hydrateLatestReleaseTag() {
  const nodes = document.querySelectorAll("[data-latest-release]");
  if (!nodes.length) return;

  try {
    const response = await fetch("https://api.github.com/repos/tylerbuilds/OSWispa/releases/latest", {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) return;

    const release = await response.json();
    const tag = String(release?.tag_name || "").trim();
    if (!tag) return;

    nodes.forEach((node) => {
      node.textContent = tag;
    });
  } catch (_) {
    // The checked-in release is the offline fallback.
  }
}

function enableReveals() {
  const nodes = document.querySelectorAll(".reveal");
  if (!nodes.length) return;

  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches || !("IntersectionObserver" in window)) {
    nodes.forEach((node) => node.classList.add("is-visible"));
    return;
  }

  const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) return;
      entry.target.classList.add("is-visible");
      observer.unobserve(entry.target);
    });
  }, { threshold: 0.12, rootMargin: "0px 0px -40px" });

  nodes.forEach((node) => observer.observe(node));
}

personalisePlatformControls();
hydrateLatestReleaseTag();
enableReveals();
