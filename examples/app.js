document.title = "Ferrum — Full HTML, CSS, and JavaScript Demo";

const hero = document.getElementById("hero");
hero.style.background = "#284f7a";

const subtitle = document.querySelector("#subtitle");
subtitle.textContent = "This subtitle was updated by JavaScript running in Boa.";

const runtime = document.getElementById("runtime-status");
runtime.textContent = "JavaScript executed and updated multiple DOM elements.";

const technologies = ["HTML", "CSS", "JavaScript"];
const status = document.querySelector("#status");
status.textContent = technologies.join(" + ") + " loaded. Click any colored panel.";
status.style.background = "#cce8d2";

const app = document.getElementById("app");
const clickableIds = [
  "parser-card",
  "layout-card",
  "paint-card",
  "runtime-card",
  "status",
];
let clickCount = 0;

for (const id of clickableIds) {
  document.getElementById(id).addEventListener("click", event => {
    event.currentTarget.style.background = "#f7c873";
  });
}

app.addEventListener("click", event => {
  clickCount += 1;
  status.textContent = "Click " + clickCount + " reached #" + event.target.id +
    " and bubbled to #" + event.currentTarget.id + ".";
  status.style.background = "#cce8d2";
  document.title = "Ferrum — " + clickCount + " interactive clicks";
});

"Ferrum page ready";
