document.documentElement.classList.add("js");

window.addEventListener("load", () => {
  document.documentElement.classList.add("loaded");

  const params = new URLSearchParams(window.location.search);
  const successBanner = document.querySelector("[data-form-success]");
  if (successBanner && params.get("sent") === "1") {
    successBanner.classList.add("is-visible");
  }
});
