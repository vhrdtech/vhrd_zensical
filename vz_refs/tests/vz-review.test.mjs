// Tests of assets/vz-review.js on a small fake page: `node --test vz_refs/tests/vz-review.test.mjs` (`just test-js`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const SCRIPT = readFileSync(new URL("../assets/vz-review.js", import.meta.url), "utf8");

class El {
  constructor(tagName, { toggle = false } = {}) {
    this.tagName = tagName;
    this.toggle = toggle;
    this.attrs = {};
    this.classes = new Set();
    this.isContentEditable = false;
    this.classList = {
      add: (c) => this.classes.add(c),
      remove: (c) => this.classes.delete(c),
      contains: (c) => this.classes.has(c),
    };
  }
  closest(sel) {
    return sel === "button.vz-review-toggle" && this.toggle ? this : null;
  }
  getAttribute(k) {
    return this.attrs[k] ?? null;
  }
  setAttribute(k, v) {
    this.attrs[k] = v;
  }
}

// a page with the toggle button, loaded into the script; `storage`: the values localStorage holds, or "blocked"
function page(storage = {}, { documentObservable = null } = {}) {
  const root = new El("HTML");
  const button = new El("BUTTON", { toggle: true });
  const body = new El("BODY");
  const listeners = {};
  const document = {
    documentElement: root,
    activeElement: body,
    querySelectorAll: (sel) => (sel === "button.vz-review-toggle" ? [button] : []),
    addEventListener: (type, fn) => (listeners[type] ??= []).push(fn),
  };
  const localStorage = {
    getItem(k) {
      if (storage === "blocked") throw new Error("SecurityError");
      return storage[k] ?? null;
    },
    setItem(k, v) {
      if (storage === "blocked") throw new Error("SecurityError");
      storage[k] = v;
    },
  };
  const window = { document$: documentObservable };
  Object.defineProperty(window, "localStorage", {
    get() {
      if (storage === "blocked") throw new Error("SecurityError");
      return localStorage;
    },
  });
  vm.runInNewContext(SCRIPT, { document, window });
  const fire = (type, props) => (listeners[type] ?? []).forEach((fn) => fn({ defaultPrevented: false, ...props }));
  return {
    root, button, body, storage, document, fire,
    on: () => root.classes.has("vz-review-on"),
    key: (key, props = {}) => fire("keydown", { key, target: document.activeElement, ...props }),
  };
}

test("the marks start hidden and the button shows once the script runs", () => {
  const p = page();
  assert.equal(p.on(), false);
  assert.ok(p.root.classes.has("vz-review-ready"));
  assert.equal(p.button.getAttribute("aria-pressed"), "false");
});

test("the button toggles the marks and remembers the choice", () => {
  const p = page();
  p.fire("click", { target: p.button });
  assert.equal(p.on(), true);
  assert.equal(p.button.getAttribute("aria-pressed"), "true");
  assert.equal(p.storage["vz-review-marks"], "on");
  p.fire("click", { target: p.button });
  assert.equal(p.on(), false);
  assert.equal(p.storage["vz-review-marks"], "off");
});

test("a click elsewhere does nothing", () => {
  const p = page();
  p.fire("click", { target: p.body });
  assert.equal(p.on(), false);
});

test("the r key toggles the marks", () => {
  const p = page();
  p.key("r");
  assert.equal(p.on(), true);
  p.key("r");
  assert.equal(p.on(), false);
});

test("the key is left alone with a modifier, a repeat, another key or when already handled", () => {
  const p = page();
  for (const props of [{ ctrlKey: true }, { metaKey: true }, { altKey: true }, { repeat: true }, { defaultPrevented: true }]) {
    p.key("r", props);
  }
  p.key("R");
  p.key("s");
  assert.equal(p.on(), false);
});

test("the key is left alone while a text field has focus", () => {
  const p = page();
  for (const tag of ["INPUT", "TEXTAREA", "SELECT"]) {
    p.document.activeElement = new El(tag);
    p.key("r");
  }
  const editable = new El("DIV");
  editable.isContentEditable = true;
  p.document.activeElement = editable;
  p.key("r");
  assert.equal(p.on(), false);
});

test("a stored choice shows the marks on load", () => {
  const p = page({ "vz-review-marks": "on" });
  assert.equal(p.on(), true);
  assert.equal(p.button.getAttribute("aria-pressed"), "true");
});

test("blocked storage still toggles, for the page only", () => {
  const p = page("blocked");
  assert.equal(p.on(), false);
  p.fire("click", { target: p.button });
  assert.equal(p.on(), true);
});

test("a page instant navigation loads gets its button's state", () => {
  let next = null;
  const p = page({ "vz-review-marks": "on" }, { documentObservable: { subscribe: (fn) => (next = fn) } });
  const later = new El("BUTTON", { toggle: true });
  p.document.querySelectorAll = (sel) => (sel === "button.vz-review-toggle" ? [later] : []);
  next();
  assert.equal(later.getAttribute("aria-pressed"), "true");
});

test("the page's badges are copied above the table of contents, and the originals hide while it shows", () => {
  const p = page();
  const made = [];
  const inner = { children: [], firstChild: null, insertBefore(el) { this.children.unshift(el); } };
  const badge = { cloneNode: () => ({ copy: true }) };
  const level = { cloneNode: () => ({ copy: "level" }) };
  const holder = () => {
    const el = { children: [], offsetParent: {}, parentNode: null, appendChild(c) { this.children.push(c); } };
    made.push(el);
    return el;
  };
  const doc = {
    ...p.document,
    querySelector: (sel) => (sel === ".md-sidebar--secondary .md-sidebar__inner" ? inner : null),
    querySelectorAll: (sel) => (sel === ".md-content .vz-sidebar-badge" ? [badge, level] : p.document.querySelectorAll(sel)),
    createElement: holder,
  };
  vm.runInNewContext(SCRIPT, { document: doc, window: {} });
  assert.equal(inner.children.length, 1);
  assert.equal(inner.children[0].className, "vz-sidebar-badges");
  assert.deepEqual(inner.children[0].children, [{ copy: true }, { copy: "level" }]);
  assert.ok(p.root.classes.has("vz-sidebar-shown"));
});

test("without a table of contents the badges stay where they are", () => {
  const p = page();
  const doc = {
    ...p.document,
    querySelector: () => null,
    querySelectorAll: (sel) => (sel === ".md-content .vz-sidebar-badge" ? [{ cloneNode: () => ({}) }] : []),
    createElement: () => assert.fail("nothing to make"),
  };
  vm.runInNewContext(SCRIPT, { document: doc, window: {} });
  assert.equal(p.root.classes.has("vz-sidebar-shown"), false);
});
