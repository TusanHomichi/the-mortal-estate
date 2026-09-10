import "./style.css";
import "./entryStyle.css";

// Never consume login credentials from navigation. Clear legacy form URLs before
// loading the game; the document also blocks native form navigation and referrers.
const url = new URL(location.href);
if ([...url.searchParams.keys()].some(key => ["username", "password"].includes(key.toLowerCase()))) {
  url.search = "";
  history.replaceState(null, "", url.pathname + url.hash);
}
void import("./main").catch(() => {
  // Controls stay disabled on module, renderer, asset or codec startup failure.
  document.body.dataset.playReady = "failed";
  const status = document.getElementById("connection");
  if (status) status.textContent = "The game could not load. Reload to try again.";
});
