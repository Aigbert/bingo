((native) => {
  delete globalThis.__bingo_native;
  globalThis.h = (kind, props, ...children) => {
    const flat = children.flat(Infinity).filter(child => child !== null && child !== undefined && typeof child !== 'boolean').map(child => typeof child === 'number' ? String(child) : child);
    return kind === 'Fragment' ? flat : {kind, ...(props ?? {}), ...(flat.length ? {children:flat} : {})};
  };
  globalThis.Fragment = 'Fragment';
  globalThis.TextEncoder = class TextEncoder {
    get encoding() { return 'utf-8'; }
    encode(text = '') { return new Uint8Array(native.encode(String(text))); }
  };
  globalThis.TextDecoder = class TextDecoder {
    constructor(label = 'utf-8', options = {}) {
      if (!['utf-8', 'utf8'].includes(label.toLowerCase())) throw new TypeError('only UTF-8 is available');
      this.fatal = !!options.fatal;
    }
    get encoding() { return 'utf-8'; }
    decode(bytes = new Uint8Array()) {
      const view = ArrayBuffer.isView(bytes) ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength) : new Uint8Array(bytes);
      return native.decode(Array.from(view), this.fatal);
    }
  };
  globalThis.URL = class URL {
    constructor(input, base) { Object.assign(this, JSON.parse(native.url(String(input), base === undefined ? undefined : String(base)))); }
    toString() { return this.href; }
    toJSON() { return this.href; }
    static canParse(input, base) { try { new URL(input, base); return true; } catch { return false; } }
  };
  class Signal {
    #listeners = new Set();
    aborted = false;
    reason;
    onabort = null;
    addEventListener(name, listener) { if (name === 'abort') this.#listeners.add(listener); }
    removeEventListener(name, listener) { if (name === 'abort') this.#listeners.delete(listener); }
    throwIfAborted() { if (this.aborted) throw this.reason; }
    abort(reason) {
      if (this.aborted) return;
      this.aborted = true;
      this.reason = reason ?? new Error('aborted');
      const event = { type: 'abort', target: this };
      this.onabort?.(event);
      for (const listener of this.#listeners) listener(event);
      this.#listeners.clear();
    }
  }
  globalThis.AbortController = class AbortController {
    signal = new Signal();
    abort(reason) { this.signal.abort(reason); }
  };
  globalThis.crypto = Object.freeze({
    randomUUID: () => native.uuid(),
    subtle: Object.freeze({ digest: async (algorithm, bytes) => {
      if (String(algorithm?.name ?? algorithm).toUpperCase() !== 'SHA-256') throw new TypeError('only SHA-256 is available');
      const view = ArrayBuffer.isView(bytes) ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength) : new Uint8Array(bytes);
      return new Uint8Array(native.digest(Array.from(view))).buffer;
    } }),
  });
  globalThis.performance = Object.freeze({ now: () => native.now() });
})(globalThis.__bingo_native);
