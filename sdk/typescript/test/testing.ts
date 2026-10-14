// Keep the helper contract runnable under Node and the embedded author kit.
export {test} from 'node:test';
import assert from 'node:assert/strict';
export function expect(value: unknown) {
  return {
    toBe(expected: unknown) { assert.equal(value, expected); },
    toEqual(expected: unknown) { assert.deepEqual(value, expected); },
    rejects: {async toThrow(message?: string) {
      await assert.rejects(Promise.resolve(value), error => {
        if (message !== undefined) assert.ok(String(error).includes(message));
        return true;
      });
    }},
  };
}
