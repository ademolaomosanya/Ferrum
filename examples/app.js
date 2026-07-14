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

if (event && event.type === "click" && event.target) {
  event.target.style.background = "#f7c873";
  status.textContent = "Clicked #" + event.target.id + ". JavaScript handled the event.";
  status.style.background = "#f7c873";
  document.title = "Ferrum — clicked #" + event.target.id;
}

"Ferrum page ready";
