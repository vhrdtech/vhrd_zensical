// vz: turns the review marks (vz-review.css) on and off and puts the page's badges above its table of contents,
// written by `vz` (do not edit). The proofread badge (`button.vz-review-toggle`) and the `r` key toggle the class `vz-review-on` on the root element; the key is
// left alone while a text field has focus or a modifier is held. The choice is kept in localStorage, so it holds
// across pages and visits; where storage is blocked it holds for the page only. Listeners sit on the document, so
// pages that instant navigation loads later need nothing but their button's state.
(function () {
  var KEY = "vz-review-marks";
  var root = document.documentElement;

  function stored() {
    try {
      return window.localStorage.getItem(KEY) === "on";
    } catch (e) {
      return false;
    }
  }
  function store(on) {
    try {
      window.localStorage.setItem(KEY, on ? "on" : "off");
    } catch (e) {
      // storage blocked: the choice holds for this page only
    }
  }
  function isOn() {
    return root.classList.contains("vz-review-on");
  }
  function show(on) {
    if (on) root.classList.add("vz-review-on");
    else root.classList.remove("vz-review-on");
    var buttons = document.querySelectorAll("button.vz-review-toggle");
    for (var i = 0; i < buttons.length; i++) buttons[i].setAttribute("aria-pressed", on ? "true" : "false");
  }
  function toggle() {
    var on = !isOn();
    show(on);
    store(on);
  }
  function typing(el) {
    if (!el) return false;
    var tag = (el.tagName || "").toUpperCase();
    return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable === true;
  }

  document.addEventListener("click", function (e) {
    var t = e.target;
    if (t && t.closest && t.closest("button.vz-review-toggle")) toggle();
  });
  document.addEventListener("keydown", function (e) {
    if (e.key !== "r" || e.ctrlKey || e.metaKey || e.altKey || e.repeat || e.defaultPrevented) return;
    if (typing(e.target) || typing(document.activeElement)) return;
    toggle();
  });

  // Copies the page's badges (`.vz-sidebar-badge`: the proofread badge, and a theme's own such as a reading level)
  // to the top of the table of contents ("On this page"), and hides the originals while that sidebar shows.
  function badgesToSidebar() {
    if (typeof document.querySelector !== "function") return;
    var inner = document.querySelector(".md-sidebar--secondary .md-sidebar__inner");
    var old = document.querySelector(".vz-sidebar-badges");
    if (old && old.parentNode) old.parentNode.removeChild(old);
    root.classList.remove("vz-sidebar-shown");
    var badges = document.querySelectorAll(".md-content .vz-sidebar-badge");
    if (!inner || badges.length === 0) return;
    var holder = document.createElement("div");
    holder.className = "vz-sidebar-badges";
    for (var i = 0; i < badges.length; i++) holder.appendChild(badges[i].cloneNode(true));
    inner.insertBefore(holder, inner.firstChild);
    if (holder.offsetParent !== null) root.classList.add("vz-sidebar-shown");
  }

  root.classList.add("vz-review-ready");
  badgesToSidebar();
  show(stored());
  if (window.document$ && typeof window.document$.subscribe === "function") {
    window.document$.subscribe(function () {
      badgesToSidebar();
      show(isOn());
    });
  }
})();
