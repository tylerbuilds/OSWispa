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
