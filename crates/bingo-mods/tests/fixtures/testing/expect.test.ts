import { test, describe, expect } from 'bingo/testing';
describe('expect', () => {
  test('matchers', async () => {
    expect(1).toBe(1);
    expect({a:[1,2]}).toEqual({a:[1,2]});
    expect({a:1,b:2}).toMatchObject({a:1});
    expect(['hello']).toContain('hello');
    expect('hello').not.toContain('goodbye');
    expect(() => { throw new Error('boom'); }).toThrow('boom');
    await expect(Promise.reject(new Error('boom'))).rejects.toThrow('boom');
  });
  test('wrong', () => expect(1).toBe(2));
  test.skip('later', () => {});
});
