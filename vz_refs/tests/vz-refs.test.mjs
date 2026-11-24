// Tests of assets/refs.js against a small fake DOM: `node --test crates/sl_book_refs/tests/` (the Python tests
// of spl_book_ext run it).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const SCRIPT = readFileSync(new URL("../assets/refs.js", import.meta.url), "utf8");

class El {
  constructor(id, { ref = false, card = null } = {}) {
    this.id = id;
    this.ref = ref;
    this.card = card;
    this.parent = null;
    this.attrs = ref ? { "data-spl-card": card } : {};
    this.classes = new Set();
    this.style = {};
    this.offsetWidth = 100;
    this.offsetHeight = 50;
    this.classList = { add: (c) => this.classes.add(c), remove: (c) => this.classes.delete(c) };
  }
  closest() {
    for (let n = this; n; n = n.parent) if (n.ref) return n;
    return null;
  }
  contains(node) {
    for (let n = node; n; n = n.parent) if (n === this) return true;
    return false;
  }
  getAttribute(k) {
    return this.attrs[k] ?? null;
  }
  setAttribute(k, v) {
    this.attrs[k] = v;
  }
  getBoundingClientRect() {
    return { left: 10, top: 10, bottom: 30 };
  }
  get isOpen() {
    return this.classes.has("spl-open");
  }
}

// a page with one reference and its card, loaded into the script
function page() {
  const ref = new El("ref", { ref: true, card: "c1" });
  const inner = new El("code");
  inner.parent = ref;
  const card = new El("c1");
  const other = new El("other");
  const listeners = {};
  let selection = { isCollapsed: true, rangeCount: 0, containsNode: () => false };
  const document = {
    documentElement: { clientWidth: 800 },
    getElementById: (id) => (id === "c1" ? card : null),
    querySelectorAll: () => [],
    addEventListener: (type, fn) => (listeners[type] ??= []).push(fn),
  };
  const window = { innerHeight: 600, addEventListener() {}, getSelection: () => selection };
  const timers = [];
  vm.runInNewContext(SCRIPT, {
    document,
    window,
    setTimeout: (fn) => timers.push(fn) && timers.length,
    clearTimeout() {},
  });
  const fire = (type, props) => (listeners[type] ?? []).forEach((fn) => fn({ preventDefault() {}, ...props }));
  return {
    ref, inner, card, other, fire,
    select: (holds) => {
      selection = { isCollapsed: false, rangeCount: 1, containsNode: (n) => holds.includes(n) };
      fire("selectionchange", {});
    },
    clear: () => {
      selection = { isCollapsed: true, rangeCount: 0, containsNode: () => false };
      fire("selectionchange", {});
    },
  };
}

test("hovering a reference opens its card", () => {
  const p = page();
  p.fire("mouseover", { target: p.inner, buttons: 0 });
  assert.equal(p.card.isOpen, true);
});

test("no card opens while the mouse button is held down", () => {
  const p = page();
  p.fire("pointerdown", { target: p.other, pointerType: "mouse", button: 0 });
  p.fire("mouseover", { target: p.inner, buttons: 1 });
  assert.equal(p.card.isOpen, false);
  p.fire("pointerup", { target: p.other });
  p.fire("mouseover", { target: p.inner, buttons: 0 });
  assert.equal(p.card.isOpen, true);
});

test("no card opens while a selection holds the reference", () => {
  const p = page();
  p.select([p.ref]);
  p.fire("mouseover", { target: p.inner, buttons: 0 });
  assert.equal(p.card.isOpen, false);
});

test("a selection elsewhere does not stop a card", () => {
  const p = page();
  p.select([p.other]);
  p.fire("mouseover", { target: p.inner, buttons: 0 });
  assert.equal(p.card.isOpen, true);
});

test("a selection that grows over an open card's reference closes it", () => {
  const p = page();
  p.fire("mouseover", { target: p.inner, buttons: 0 });
  assert.equal(p.card.isOpen, true);
  p.select([p.ref]);
  assert.equal(p.card.isOpen, false);
});

test("clearing the selection lets hover open the card again", () => {
  const p = page();
  p.select([p.ref]);
  p.fire("mouseover", { target: p.inner, buttons: 0 });
  assert.equal(p.card.isOpen, false);
  p.clear();
  assert.equal(p.card.isOpen, true, "the pointer still rests on the reference");
  p.fire("mouseout", { target: p.inner, relatedTarget: p.other });
  p.fire("mouseover", { target: p.inner, buttons: 0 });
  assert.equal(p.card.isOpen, true);
});

test("keyboard focus does not open a card under a selection", () => {
  const p = page();
  p.select([p.ref]);
  p.fire("focusin", { target: p.ref });
  assert.equal(p.card.isOpen, false);
  p.clear();
  p.fire("focusin", { target: p.ref });
  assert.equal(p.card.isOpen, true);
});
