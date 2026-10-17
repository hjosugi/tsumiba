// Just enough of the WebExtension API for background.js, in plain objects that
// record what happened. Not a general mock: it implements exactly what the
// extension calls.

export function makeEvent() {
  const listeners = [];
  return {
    listeners,
    addListener(fn) {
      listeners.push(fn);
    },
    removeListener(fn) {
      listeners.splice(listeners.indexOf(fn), 1);
    },
    /** Call every listener and wait for any promises they return. */
    fire(...args) {
      return Promise.all(listeners.map((l) => l(...args)));
    },
  };
}

export function createFakeChrome({ storage = {}, granted = false, requestResult = true } = {}) {
  const state = {
    storage: { ...storage },
    granted,
    requestResult,
    badge: { text: '', color: null, title: 'tsumiba' },
    badgeHistory: [],
    permissionRequests: 0,
    optionsOpened: 0,
    menus: new Map(),
    menuErrors: [],
    activeTab: { id: 1, title: 'Active tab', url: 'https://active.example/' },
  };

  const chrome = {
    state,
    runtime: {
      lastError: undefined,
      onInstalled: makeEvent(),
      onStartup: makeEvent(),
      openOptionsPage() {
        state.optionsOpened += 1;
        return Promise.resolve();
      },
    },
    contextMenus: {
      onClicked: makeEvent(),
      async removeAll() {
        state.menus.clear();
      },
      create(props, callback) {
        if (state.menus.has(props.id)) {
          const message = `Cannot create item with duplicate id ${props.id}`;
          state.menuErrors.push(message);
          chrome.runtime.lastError = { message };
        } else {
          state.menus.set(props.id, props);
        }
        callback?.();
        chrome.runtime.lastError = undefined;
        return props.id;
      },
    },
    action: {
      async setBadgeText({ text }) {
        state.badge.text = text;
        state.badgeHistory.push(text);
      },
      async setBadgeBackgroundColor({ color }) {
        state.badge.color = color;
      },
      async setBadgeTextColor() {},
      async setTitle({ title }) {
        state.badge.title = title;
      },
    },
    commands: { onCommand: makeEvent() },
    tabs: {
      async query() {
        return [state.activeTab];
      },
    },
    permissions: {
      async contains() {
        return state.granted;
      },
      async request() {
        state.permissionRequests += 1;
        state.granted = state.requestResult;
        return state.requestResult;
      },
      async remove() {
        state.granted = false;
        return true;
      },
    },
    storage: {
      local: {
        async get(keys) {
          if (keys == null) return { ...state.storage };
          const list = Array.isArray(keys) ? keys : typeof keys === 'string' ? [keys] : Object.keys(keys);
          return Object.fromEntries(list.filter((k) => k in state.storage).map((k) => [k, state.storage[k]]));
        },
        async set(update) {
          Object.assign(state.storage, update);
        },
      },
    },
  };
  return chrome;
}

/** Install a fake as the global `chrome` (and make sure `browser` is absent). */
export function installChrome(chrome) {
  delete globalThis.browser;
  globalThis.chrome = chrome;
}

export function uninstallChrome() {
  delete globalThis.chrome;
  delete globalThis.browser;
}

/** Pin `navigator.language` so message assertions do not depend on the machine. */
export function pinLanguage(language) {
  Object.defineProperty(globalThis, 'navigator', {
    value: { language, platform: 'Linux' },
    configurable: true,
    writable: true,
  });
}
